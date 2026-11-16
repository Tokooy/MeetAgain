//! Curated city-level road model, not a routable road-network extraction.
use geo::{Distance, Haversine, Point};
use geojson::{FeatureCollection, GeoJson, Value};
use petgraph::{
    algo::{astar, connected_components},
    graph::{NodeIndex, UnGraph},
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

pub const BOUNDARIES: &str = include_str!("../data/jiangsu.geojson");
pub const ROADS: &str = include_str!("../data/roads.json");
const CITY_NAMES: [&str; 13] = [
    "南京市",
    "无锡市",
    "徐州市",
    "常州市",
    "苏州市",
    "南通市",
    "连云港市",
    "淮安市",
    "盐城市",
    "扬州市",
    "镇江市",
    "泰州市",
    "宿迁市",
];

#[derive(Debug, Error)]
#[error("invalid Jiangsu graph: {0}")]
pub struct GraphError(pub String);

#[derive(Clone, Debug, Serialize)]
pub struct City {
    pub id: String,
    pub name: String,
    pub center: [f64; 2],
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
    graph: UnGraph<(), f64>,
}

fn invalid(message: impl Into<String>) -> GraphError {
    GraphError(message.into())
}

fn valid_coordinate(c: &[f64]) -> bool {
    c.len() == 2
        && c.iter().all(|v| v.is_finite())
        && (116.0..=123.0).contains(&c[0])
        && (30.0..=36.0).contains(&c[1])
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
                "polygon ring is unclosed, too short, or outside Jiangsu bounds",
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

impl RoadGraph {
    pub fn jiangsu() -> Result<Self, GraphError> {
        Self::load(BOUNDARIES, ROADS)
    }

    pub fn load(boundaries: &str, roads: &str) -> Result<Self, GraphError> {
        let GeoJson::FeatureCollection(collection) = boundaries
            .parse::<GeoJson>()
            .map_err(|e| invalid(e.to_string()))?
        else {
            return Err(invalid("expected FeatureCollection"));
        };
        if collection.features.len() != 13 {
            return Err(invalid("expected exactly 13 cities"));
        }
        let mut city_map = BTreeMap::new();
        for feature in &collection.features {
            let props = feature
                .properties
                .as_ref()
                .ok_or_else(|| invalid("missing properties"))?;
            let raw_id = props
                .get("id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| invalid("missing city id"))?;
            let id = format!("{raw_id}00");
            let index = (1..=13)
                .find(|n| id == format!("32{n:02}00"))
                .ok_or_else(|| invalid("unknown city id"))?;
            let name = props
                .get("name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| invalid("missing city name"))?;
            if name != CITY_NAMES[index - 1] {
                return Err(invalid("city id/name mismatch"));
            }
            let center: [f64; 2] = serde_json::from_value(
                props
                    .get("cp")
                    .cloned()
                    .ok_or_else(|| invalid("missing center"))?,
            )
            .map_err(|e| invalid(e.to_string()))?;
            if !valid_coordinate(&center) {
                return Err(invalid("invalid center"));
            }
            let geometry = &feature
                .geometry
                .as_ref()
                .ok_or_else(|| invalid("missing geometry"))?
                .value;
            match geometry {
                Value::Polygon(rings) => validate_polygon(rings)?,
                Value::MultiPolygon(polygons) if !polygons.is_empty() => {
                    for rings in polygons {
                        validate_polygon(rings)?;
                    }
                }
                _ => return Err(invalid("expected Polygon or nonempty MultiPolygon")),
            }
            if city_map
                .insert(
                    id.clone(),
                    City {
                        id,
                        name: name.to_owned(),
                        center,
                    },
                )
                .is_some()
            {
                return Err(invalid("duplicate city"));
            }
        }
        let cities: Vec<City> = city_map.into_values().collect();
        let indices: BTreeMap<_, _> = cities
            .iter()
            .enumerate()
            .map(|(i, c)| (c.id.as_str(), i))
            .collect();
        let edges: Vec<[String; 2]> =
            serde_json::from_str(roads).map_err(|e| invalid(e.to_string()))?;
        let mut graph = UnGraph::new_undirected();
        for _ in &cities {
            graph.add_node(());
        }
        let mut seen = BTreeSet::new();
        let mut weights = Vec::new();
        for [source, target] in edges {
            let a = *indices
                .get(source.as_str())
                .ok_or_else(|| invalid("unknown road endpoint"))?;
            let b = *indices
                .get(target.as_str())
                .ok_or_else(|| invalid("unknown road endpoint"))?;
            if a == b || !seen.insert((a.min(b), a.max(b))) {
                return Err(invalid("self-loop or duplicate undirected edge"));
            }
            let distance = Haversine
                .distance(Point::from(cities[a].center), Point::from(cities[b].center))
                / 1000.0;
            if !distance.is_finite() || distance <= 0.0 {
                return Err(invalid("invalid edge weight"));
            }
            graph.add_edge(NodeIndex::new(a), NodeIndex::new(b), distance);
            weights.push(Road {
                source,
                target,
                distance_km: distance,
            });
        }
        if seen.len() != 23 || connected_components(&graph) != 1 {
            return Err(invalid("expected 23 edges and one connected component"));
        }
        weights.sort_by(|a, b| (&a.source, &a.target).cmp(&(&b.source, &b.target)));
        Ok(Self {
            cities,
            roads: weights,
            boundaries: collection,
            boundary_sha256: format!("{:x}", Sha256::digest(boundaries.as_bytes())),
            graph,
        })
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
