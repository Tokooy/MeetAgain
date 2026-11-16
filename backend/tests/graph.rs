use meetagain::graph::{BOUNDARIES, ROADS, RoadGraph};
use serde_json::Value;

#[test]
fn graph_has_thirteen_cities_twenty_three_symmetric_weighted_edges() {
    let g = RoadGraph::jiangsu().unwrap();
    assert_eq!((g.cities.len(), g.roads.len()), (13, 23));
    assert_eq!(g.cities[0].name, "南京市");
    let mut degree_sum = 0;
    for i in 0..13 {
        for (j, distance) in g.neighbors(i) {
            degree_sum += 1;
            assert!(distance > 0.0 && distance < 500.0);
            assert!(g.neighbors(j).contains(&(i, distance)));
        }
        for home in 0..13 {
            let mut current = i;
            let mut steps = 0;
            while current != home {
                let next = g.step_home(current, home).unwrap();
                assert!(g.neighbors(current).iter().any(|(j, _)| *j == next));
                current = next;
                steps += 1;
                assert!(steps < 13, "shortest path must terminate");
            }
        }
    }
    assert_eq!(degree_sum, 46);
}

#[test]
fn loader_rejects_missing_duplicate_or_mislabeled_cities() {
    let original: Value = serde_json::from_str(BOUNDARIES).unwrap();
    for mutation in 0..3 {
        let mut data = original.clone();
        let features = data["features"].as_array_mut().unwrap();
        match mutation {
            0 => {
                features.pop();
            }
            1 => {
                features[1] = features[0].clone();
            }
            _ => {
                features[0]["properties"]["name"] = "不存在市".into();
            }
        }
        assert!(RoadGraph::load(&data.to_string(), ROADS).is_err());
    }
}

#[test]
fn loader_rejects_corrupt_coordinates_and_non_polygon_geometry() {
    let original: Value = serde_json::from_str(BOUNDARIES).unwrap();
    for mutation in 0..3 {
        let mut data = original.clone();
        match mutation {
            0 => data["features"][0]["properties"]["cp"] = serde_json::json!([10, 90]),
            1 => {
                data["features"][0]["geometry"] =
                    serde_json::json!({"type":"Point","coordinates":[120,33]})
            }
            _ => {
                data["features"][0]["geometry"]["coordinates"][0][0] = serde_json::json!([120, 90])
            }
        }
        assert!(RoadGraph::load(&data.to_string(), ROADS).is_err());
    }
}

#[test]
fn loader_rejects_bad_roads_instead_of_silently_dropping_them() {
    for edge in [
        ["320100", "320100"],
        ["320100", "999999"],
        ["321100", "320100"],
    ] {
        let mut edges: Vec<[String; 2]> = serde_json::from_str(ROADS).unwrap();
        edges.push(edge.map(str::to_owned));
        assert!(RoadGraph::load(BOUNDARIES, &serde_json::to_string(&edges).unwrap()).is_err());
    }
    assert!(RoadGraph::load(BOUNDARIES, "[]").is_err());
}
