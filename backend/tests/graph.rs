use flate2::{Compression, write::GzEncoder};
use meetagain::graph::RoadGraph;
use serde_json::{Value, json};
use std::{fs, io::Write, path::PathBuf};

#[test]
fn bundled_graph_contains_only_the_three_selected_cities() {
    let graph = RoadGraph::bundled().unwrap();
    assert_eq!(
        graph
            .cities
            .iter()
            .map(|c| c.id.as_str())
            .collect::<Vec<_>>(),
        ["320100", "321000", "321100"]
    );
    assert_eq!(graph.roads.len(), 3);
    for (index, city) in graph.cities.iter().enumerate() {
        assert!(city.street_segments > 10_000);
        assert!(graph.streets(&city.id).is_some());
        for (neighbor, distance) in graph.neighbors(index) {
            assert!(distance > 0.0 && distance < 150.0);
            assert!(graph.neighbors(neighbor).contains(&(index, distance)));
            assert_eq!(graph.step_home(index, neighbor), Some(neighbor));
        }
    }
    assert!(graph.streets("320400").is_none());
}

struct MapFixture(PathBuf);

impl MapFixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("meetagain-map-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn write_city(&self, id: &str, longitude: f64, relation: u64) {
        let city = self.0.join("cities").join(id);
        fs::create_dir_all(&city).unwrap();
        let boundary = json!({
            "type":"Feature",
            "properties":{
                "city_id":id, "name":id, "center":[longitude,32.0], "osm_relation":relation,
                "source_url":"https://example.com/roads.osm.pbf", "osm_timestamp":null
            },
            "geometry":{
                "type":"Polygon",
                "coordinates":[[[longitude-0.1,31.9],[longitude+0.1,31.9],
                    [longitude+0.1,32.1],[longitude-0.1,32.1],[longitude-0.1,31.9]]]
            }
        });
        fs::write(city.join("boundary.geojson"), boundary.to_string()).unwrap();
        let data = json!({"type":"FeatureCollection","features":[{
            "type":"Feature",
            "properties":{"class":"local","highway":"residential","osm_id":123},
            "geometry":{"type":"LineString","coordinates":[[longitude,32.0],[longitude+0.02,32.02]]}
        }]});
        let file = fs::File::create(city.join("streets.geojson.gz")).unwrap();
        let mut gzip = GzEncoder::new(file, Compression::default());
        write!(gzip, "{data}").unwrap();
        gzip.finish().unwrap();
    }

    fn manifest(&self, data: Value) {
        fs::write(self.0.join("map.json"), data.to_string()).unwrap();
    }
}

impl Drop for MapFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn adding_a_city_works_using_only_map_data_and_changes_the_version() {
    let fixture = MapFixture::new();
    fixture.write_city("alpha", 118.0, 1);
    fixture.write_city("beta", 119.0, 2);
    fixture.write_city("new-city", 120.0, 3);
    fixture.manifest(json!({"cities":["alpha","beta"],"links":[["alpha","beta"]]}));
    let original = RoadGraph::load_dir(&fixture.0).unwrap();
    assert!(original.streets("new-city").is_none());

    fixture.manifest(json!({
        "cities":["alpha","beta","new-city"],
        "links":[["alpha","beta"],["beta","new-city"]]
    }));
    let extended = RoadGraph::load_dir(&fixture.0).unwrap();
    assert_eq!(extended.cities.len(), 3);
    assert_eq!(extended.step_home(0, 2), Some(1));
    assert!(extended.streets("new-city").is_some());
    assert_ne!(extended.version, original.version);
}

#[test]
fn invalid_manifest_links_are_rejected() {
    let fixture = MapFixture::new();
    fixture.write_city("alpha", 118.0, 1);
    fixture.write_city("beta", 119.0, 2);
    for links in [
        json!([]),
        json!([["alpha", "gamma"]]),
        json!([["alpha", "alpha"]]),
        json!([["alpha", "beta"], ["beta", "alpha"]]),
    ] {
        fixture.manifest(json!({"cities":["alpha","beta"],"links":links}));
        assert!(RoadGraph::load_dir(&fixture.0).is_err());
    }
    fixture.manifest(json!({"cities":["alpha","../beta"],"links":[]}));
    assert!(RoadGraph::load_dir(&fixture.0).is_err());
}

#[test]
fn missing_or_malformed_city_files_fail_validation() {
    let fixture = MapFixture::new();
    fixture.write_city("alpha", 118.0, 1);
    fixture.manifest(json!({"cities":["alpha","beta"],"links":[["alpha","beta"]]}));
    assert!(RoadGraph::load_dir(&fixture.0).is_err());
    fixture.write_city("beta", 119.0, 2);
    let city = fixture.0.join("cities/beta");
    fs::write(city.join("streets.geojson.gz"), "not a gzip file").unwrap();
    assert!(RoadGraph::load_dir(&fixture.0).is_err());

    fixture.write_city("beta", 119.0, 2);
    let path = city.join("boundary.geojson");
    let mut data: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    data["geometry"]["coordinates"][0][0] = json!([120, 90]);
    fs::write(path, data.to_string()).unwrap();
    assert!(RoadGraph::load_dir(&fixture.0).is_err());
}
