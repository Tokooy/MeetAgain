//! The simulation uses city links for movement and serves OSM streets as a separate map layer.
use axum::body::Bytes;
use flate2::read::GzDecoder;
use geo::{Distance, Haversine, Point};
use geojson::{Feature, FeatureCollection, GeoJson, Value};
use petgraph::{
    algo::{astar, connected_components},
    graph::{NodeIndex, UnGraph},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Read,
    path::{Path, PathBuf},
    sync::OnceLock,
};
use thiserror::Error;

pub const OSM_ATTRIBUTION: &str = "© OpenStreetMap contributors";
pub const OSM_COPYRIGHT_URL: &str = "https://www.openstreetmap.org/copyright";
const MAX_STREET_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Clone, Debug, Error)]
#[error("invalid map: {0}")]
pub struct GraphError(String);

#[derive(Clone, Debug, Serialize)]
pub struct City {
    pub id: String,
    pub name: String,
    pub center: [f64; 2],
    pub osm_relation: u64,
    pub source_url: String,
    pub osm_timestamp: Option<String>,
    pub street_segments: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct Road {
    pub source: String,
    pub target: String,
    pub distance_km: f64,
}

#[derive(Clone, Debug)]
pub struct RoadGraph {
    pub cities: Vec<City>,
    pub roads: Vec<Road>,
    pub boundaries: FeatureCollection,
    pub boundary_sha256: String,
    pub version: String,
    streets: BTreeMap<String, Bytes>,
    graph: UnGraph<(), f64>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    cities: Vec<String>,
    links: Vec<[String; 2]>,
}

#[derive(Deserialize)]
struct BoundaryProperties {
    city_id: String,
    name: String,
    center: [f64; 2],
    osm_relation: u64,
    source_url: String,
    osm_timestamp: Option<String>,
}

fn invalid(message: impl Into<String>) -> GraphError {
    GraphError(message.into())
}

fn city_id_valid(id: &str) -> bool {
    let alphanumeric = |byte: u8| byte.is_ascii_lowercase() || byte.is_ascii_digit();
    !id.is_empty()
        && id.len() <= 32
        && alphanumeric(id.as_bytes()[0])
        && id
            .bytes()
            .all(|byte| alphanumeric(byte) || matches!(byte, b'_' | b'-'))
}

fn valid_coordinate(c: &[f64]) -> bool {
    c.len() == 2 && (-180.0..=180.0).contains(&c[0]) && (-85.0..=85.0).contains(&c[1])
}

fn validate_polygon(rings: &[Vec<Vec<f64>>]) -> Result<(), GraphError> {
    if rings.is_empty() {
        return Err(invalid("polygon has no rings"));
    }
    for ring in rings {
        if ring.len() < 4
            || ring.first() != ring.last()
            || !ring.iter().all(|c| valid_coordinate(c))
        {
            return Err(invalid(
                "polygon ring is unclosed, too short, or has invalid coordinates",
            ));
        }
        let area: f64 = ring
            .windows(2)
            .map(|p| p[0][0] * p[1][1] - p[1][0] * p[0][1])
            .sum();
        if area.abs() < 1e-12 {
            return Err(invalid("degenerate polygon ring"));
        }
    }
    Ok(())
}

fn local_error(path: &Path, error: impl std::fmt::Display) -> GraphError {
    invalid(format!("{}: {error}", path.display()))
}

fn load_boundary(path: &Path, id: &str) -> Result<(Feature, BoundaryProperties), GraphError> {
    let input = fs::read_to_string(path).map_err(|error| local_error(path, error))?;
    let GeoJson::Feature(mut boundary) = input
        .parse::<GeoJson>()
        .map_err(|error| local_error(path, error))?
    else {
        return Err(local_error(path, "expected one GeoJSON Feature"));
    };
    let properties: BoundaryProperties = serde_json::from_value(serde_json::Value::Object(
        boundary
            .properties
            .take()
            .ok_or_else(|| local_error(path, "missing properties"))?,
    ))
    .map_err(|error| local_error(path, error))?;
    if properties.city_id != id
        || properties.name.trim().is_empty()
        || !valid_coordinate(&properties.center)
        || properties.osm_relation == 0
        || !(properties.source_url.starts_with("https://")
            || properties.source_url.starts_with("http://"))
    {
        return Err(local_error(
            path,
            "invalid city ID, name, center, OSM relation, or extract URL",
        ));
    }
    let geometry = boundary
        .geometry
        .take()
        .ok_or_else(|| local_error(path, "missing geometry"))?;
    match &geometry.value {
        Value::Polygon(rings) => {
            validate_polygon(rings).map_err(|error| local_error(path, error))?
        }
        Value::MultiPolygon(polygons) if !polygons.is_empty() => {
            for rings in polygons {
                validate_polygon(rings).map_err(|error| local_error(path, error))?;
            }
        }
        _ => {
            return Err(local_error(
                path,
                "expected Polygon or nonempty MultiPolygon",
            ));
        }
    }
    let feature = Feature {
        geometry: Some(geometry),
        properties: Some(serde_json::Map::from_iter([
            ("city_id".into(), serde_json::json!(id)),
            ("name".into(), serde_json::json!(properties.name)),
        ])),
        ..Feature::default()
    };
    Ok((feature, properties))
}

fn load_streets(path: &Path) -> Result<(Bytes, usize), GraphError> {
    let file = fs::File::open(path).map_err(|error| local_error(path, error))?;
    let mut data = Vec::new();
    GzDecoder::new(file)
        .take(MAX_STREET_BYTES + 1)
        .read_to_end(&mut data)
        .map_err(|error| local_error(path, error))?;
    if data.len() as u64 > MAX_STREET_BYTES {
        return Err(local_error(
            path,
            "street GeoJSON exceeds 64 MiB uncompressed",
        ));
    }
    let collection: GeoJson =
        serde_json::from_slice(&data).map_err(|error| local_error(path, error))?;
    let GeoJson::FeatureCollection(collection) = collection else {
        return Err(local_error(path, "expected a street FeatureCollection"));
    };
    if collection.features.is_empty() {
        return Err(local_error(path, "expected at least one street"));
    }
    for feature in &collection.features {
        let Some(Value::LineString(line)) = feature.geometry.as_ref().map(|g| &g.value) else {
            return Err(local_error(path, "a street is missing its LineString"));
        };
        if line.len() < 2
            || !line.iter().all(|point| valid_coordinate(point))
            || line.windows(2).all(|pair| pair[0] == pair[1])
        {
            return Err(local_error(path, "a street has invalid coordinates"));
        }
        let class = feature.property("class").and_then(|value| value.as_str());
        let highway = feature.property("highway").and_then(|value| value.as_str());
        let id = feature.property("osm_id").and_then(|value| value.as_u64());
        if !matches!(class, Some("major" | "arterial" | "local"))
            || highway.is_none_or(str::is_empty)
            || id.is_none_or(|id| id == 0)
        {
            return Err(local_error(
                path,
                "a street has invalid class, highway or OSM way ID",
            ));
        }
    }
    Ok((Bytes::from(data), collection.features.len()))
}

fn bundled_map_dir() -> PathBuf {
    if let Ok(executable) = std::env::current_exe()
        && let Some(parent) = executable.parent()
    {
        let alongside = parent.join("map");
        if alongside.join("map.json").is_file() {
            return alongside;
        }
    }
    Path::new(env!("CARGO_MANIFEST_DIR")).join("data/map")
}

impl RoadGraph {
    pub fn bundled() -> Result<Self, GraphError> {
        static BUNDLED: OnceLock<Result<RoadGraph, GraphError>> = OnceLock::new();
        BUNDLED
            .get_or_init(|| Self::load_dir(bundled_map_dir()))
            .clone()
    }

    pub fn load_dir(directory: impl AsRef<Path>) -> Result<Self, GraphError> {
        let directory = directory.as_ref();
        let manifest_path = directory.join("map.json");
        let manifest: Manifest = serde_json::from_slice(
            &fs::read(&manifest_path).map_err(|error| local_error(&manifest_path, error))?,
        )
        .map_err(|error| local_error(&manifest_path, error))?;
        let mut ids = manifest.cities;
        if ids.len() < 2 || ids.iter().any(|id| !city_id_valid(id)) {
            return Err(invalid("map.json requires at least two valid city IDs"));
        }
        ids.sort_unstable();
        if ids.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(invalid("duplicate city ID in map.json"));
        }
        let mut cities = Vec::new();
        let mut boundaries = FeatureCollection::default();
        let mut streets = BTreeMap::new();
        let mut relations = BTreeSet::new();
        for id in ids {
            let city_folder = directory.join("cities").join(&id);
            let (boundary, properties) = load_boundary(&city_folder.join("boundary.geojson"), &id)?;
            if !relations.insert(properties.osm_relation) {
                return Err(invalid(format!(
                    "OSM relation {} is listed for multiple cities",
                    properties.osm_relation
                )));
            }
            let (street_json, street_segments) =
                load_streets(&city_folder.join("streets.geojson.gz"))?;
            streets.insert(id.clone(), street_json);
            boundaries.features.push(boundary);
            cities.push(City {
                id,
                name: properties.name,
                center: properties.center,
                osm_relation: properties.osm_relation,
                source_url: properties.source_url,
                osm_timestamp: properties.osm_timestamp,
                street_segments,
            });
        }
        let mut graph = UnGraph::new_undirected();
        let indices: BTreeMap<_, _> = cities
            .iter()
            .map(|city| (city.id.as_str(), graph.add_node(())))
            .collect();
        let mut links = manifest.links;
        for [source, target] in &mut links {
            if source > target {
                std::mem::swap(source, target);
            }
        }
        links.sort_unstable();
        if links.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(invalid("duplicate undirected city link"));
        }
        let mut roads = Vec::new();
        for [source, target] in links {
            let a = *indices
                .get(source.as_str())
                .ok_or_else(|| invalid(format!("unknown link endpoint {source}")))?;
            let b = *indices
                .get(target.as_str())
                .ok_or_else(|| invalid(format!("unknown link endpoint {target}")))?;
            if a == b {
                return Err(invalid(format!("self-link for {source}")));
            }
            let distance = Haversine.distance(
                Point::from(cities[a.index()].center),
                Point::from(cities[b.index()].center),
            ) / 1000.0;
            if !distance.is_finite() || distance <= 0.0 {
                return Err(invalid(format!("invalid distance for {source}–{target}")));
            }
            graph.add_edge(a, b, distance);
            roads.push(Road {
                source,
                target,
                distance_km: distance,
            });
        }
        if connected_components(&graph) != 1 {
            return Err(invalid("city links must form one connected component"));
        }
        let boundary_sha256 = format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&boundaries).expect("validated boundaries"))
        );
        let mut hash = Sha256::new();
        hash.update(boundary_sha256.as_bytes());
        hash.update(serde_json::to_vec(&(&cities, &roads)).expect("validated graph metadata"));
        for data in streets.values() {
            hash.update(Sha256::digest(data));
        }
        Ok(Self {
            cities,
            roads,
            boundaries,
            boundary_sha256,
            version: format!("osm-map-v1-{:x}", hash.finalize()),
            streets,
            graph,
        })
    }

    pub fn streets(&self, city_id: &str) -> Option<Bytes> {
        self.streets.get(city_id).cloned()
    }

    pub fn neighbors(&self, city: usize) -> Vec<(usize, f64)> {
        use petgraph::visit::EdgeRef;
        let mut neighbors: Vec<_> = self
            .graph
            .edges(NodeIndex::new(city))
            .map(|edge| (edge.target().index(), *edge.weight()))
            .collect();
        neighbors.sort_by_key(|(index, _)| *index);
        neighbors
    }

    pub fn step_home(&self, city: usize, home: usize) -> Option<usize> {
        astar(
            &self.graph,
            NodeIndex::new(city),
            |n| n.index() == home,
            |e| *e.weight(),
            |_| 0.0,
        )
        .and_then(|(_, path)| path.get(1).map(|n| n.index()))
    }
}
