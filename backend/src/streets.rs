//! Find street identities in the clipped OSM geometry and sample positions along it.
use geo::{Distance, Haversine, Point};
use petgraph::unionfind::UnionFind;
use rand::Rng;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct StreetPoint {
    pub street_id: u64,
    pub osm_way_id: u64,
    pub street_name: Option<String>,
    pub coordinates: [f64; 2],
}

#[derive(Debug)]
pub(crate) struct StreetLine {
    pub way_id: u64,
    pub name: Option<String>,
    pub reference: Option<String>,
    pub coordinates: Vec<[f64; 2]>,
}

impl StreetLine {
    fn label(&self) -> Option<(bool, &str)> {
        // A name and a route reference are separate namespaces.
        self.name.as_deref().map(|name| (true, name)).or_else(|| {
            self.reference
                .as_deref()
                .map(|reference| (false, reference))
        })
    }
}

#[derive(Clone, Debug)]
struct StreetIdentity {
    id: u64,
    name: Option<String>,
}

#[derive(Debug)]
struct Segment {
    street: usize,
    way_id: u64,
    start: [f64; 2],
    end: [f64; 2],
    cumulative_m: f64,
}

#[derive(Debug)]
pub(crate) struct StreetCatalog {
    by_city: Vec<Vec<Segment>>,
    streets: Vec<Option<StreetIdentity>>,
}

pub(crate) fn distance_m(a: [f64; 2], b: [f64; 2]) -> f64 {
    Haversine.distance(Point::from(a), Point::from(b))
}

impl StreetCatalog {
    pub(crate) fn new(lines_by_city: Vec<Vec<StreetLine>>) -> Self {
        let lines = lines_by_city.iter().flatten();
        let count = lines.clone().count();
        let mut groups = UnionFind::new(count);
        let mut by_way = HashMap::new();
        let mut by_named_vertex = HashMap::new();

        for (index, line) in lines.clone().enumerate() {
            if let Some(previous) = by_way.insert(line.way_id, index) {
                groups.union(index, previous);
            }
            if let Some(label) = line.label() {
                for [lon, lat] in &line.coordinates {
                    // Match vertices at the precision of the OSM extract and clipped geometry.
                    let vertex = ((lon * 1e7).round() as i32, (lat * 1e7).round() as i32);
                    if let Some(previous) = by_named_vertex.insert((label, vertex), index) {
                        groups.union(index, previous);
                    }
                }
            }
        }

        let mut streets: Vec<Option<StreetIdentity>> = vec![None; count];
        for (index, line) in lines.enumerate() {
            let slot = &mut streets[groups.find_mut(index)];
            if slot.as_ref().is_none_or(|street| line.way_id < street.id) {
                *slot = Some(StreetIdentity {
                    id: line.way_id,
                    name: line.label().map(|(_, label)| label.to_owned()),
                });
            }
        }

        let mut next_line = 0;
        let by_city = lines_by_city
            .into_iter()
            .map(|lines| {
                let mut cumulative_m = 0.0;
                let mut segments = Vec::new();
                for line in lines {
                    let street = groups.find_mut(next_line);
                    next_line += 1;
                    for pair in line.coordinates.windows(2) {
                        let [start, end] = [pair[0], pair[1]];
                        let length = distance_m(start, end);
                        if length > 0.0 {
                            cumulative_m += length;
                            segments.push(Segment {
                                street,
                                way_id: line.way_id,
                                start,
                                end,
                                cumulative_m,
                            });
                        }
                    }
                }
                segments
            })
            .collect();
        Self { by_city, streets }
    }

    pub(crate) fn sample(&self, city: usize, rng: &mut impl Rng) -> StreetPoint {
        let segments = &self.by_city[city];
        let draw = rng.random::<f64>() * segments.last().expect("validated streets").cumulative_m;
        let index = segments
            .partition_point(|segment| segment.cumulative_m <= draw)
            .min(segments.len() - 1);
        let segment = &segments[index];
        let start_m = index
            .checked_sub(1)
            .map_or(0.0, |i| segments[i].cumulative_m);
        let fraction = (draw - start_m) / (segment.cumulative_m - start_m);
        let coordinates = std::array::from_fn(|i| {
            segment.start[i] + (segment.end[i] - segment.start[i]) * fraction
        });
        let street = self.streets[segment.street]
            .as_ref()
            .expect("indexed street");
        StreetPoint {
            street_id: street.id,
            osm_way_id: segment.way_id,
            street_name: street.name.clone(),
            coordinates,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;

    fn line(id: u64, name: Option<&str>, start: f64, end: f64) -> StreetLine {
        StreetLine {
            way_id: id,
            name: name.map(str::to_owned),
            reference: None,
            coordinates: vec![[start, 32.0], [end, 32.0]],
        }
    }

    #[test]
    fn joins_clipped_ways_and_connected_names_without_joining_disconnected_names() {
        let graph = StreetCatalog::new(vec![
            vec![line(9, Some("人民路"), 119.0, 119.1)],
            vec![line(5, Some("人民路"), 119.1, 119.2)],
            vec![line(7, Some("人民路"), 119.2001, 119.3)],
            vec![line(11, Some("建设路"), 119.1, 119.2)],
            vec![line(90, None, 119.3, 119.4)],
            vec![line(90, None, 119.4, 119.5)],
        ]);
        let mut rng = ChaCha8Rng::seed_from_u64(17);
        assert_eq!(graph.sample(0, &mut rng).street_id, 5);
        assert_eq!(graph.sample(1, &mut rng).street_id, 5);
        assert_eq!(graph.sample(2, &mut rng).street_id, 7);
        assert_eq!(graph.sample(3, &mut rng).street_id, 11);
        assert_eq!(graph.sample(4, &mut rng).street_id, 90);
        assert_eq!(graph.sample(5, &mut rng).street_id, 90);
        let sample = graph.sample(1, &mut rng);
        assert_eq!(sample.osm_way_id, 5);
        assert_eq!(sample.street_name.as_deref(), Some("人民路"));
        assert_eq!(sample.coordinates[1], 32.0);
        assert!((119.1..=119.2).contains(&sample.coordinates[0]));
    }
}
