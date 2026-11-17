use flate2::{Compression, write::GzEncoder};
use meetagain::graph::RoadGraph;
use serde_json::{Value, json};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

pub struct MapFixture(PathBuf);

impl MapFixture {
    pub fn new() -> Self {
        let path = std::env::temp_dir().join(format!("meetagain-map-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    pub fn path(&self) -> &Path {
        &self.0
    }

    pub fn short_streets() -> Self {
        let fixture = Self::new();
        fixture.write_city("alpha", 118.0, 1);
        fixture.write_city("beta", 119.0, 2);
        fixture.write_city("gamma", 120.0, 3);
        fixture.manifest(json!({
            "cities":["alpha","beta","gamma"],
            "links":[["alpha","beta"],["beta","gamma"]]
        }));
        fixture
    }

    pub fn graph(&self) -> RoadGraph {
        RoadGraph::load_dir(self.path()).unwrap()
    }

    pub fn write_city(&self, id: &str, longitude: f64, relation: u64) {
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
        // A short street makes same-street meetings deterministic within a city.
        let data = json!({"type":"FeatureCollection","features":[{
            "type":"Feature",
            "properties":{"class":"local","highway":"residential","osm_id":relation+100, "name":"Fixture Street"},
            "geometry":{"type":"LineString","coordinates":[[longitude,32.0],[longitude+0.0003,32.0]]}
        }]});
        let file = fs::File::create(city.join("streets.geojson.gz")).unwrap();
        let mut gzip = GzEncoder::new(file, Compression::default());
        write!(gzip, "{data}").unwrap();
        gzip.finish().unwrap();
    }

    pub fn manifest(&self, data: Value) {
        fs::write(self.0.join("map.json"), data.to_string()).unwrap();
    }
}
impl Drop for MapFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
