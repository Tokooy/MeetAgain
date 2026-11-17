//! Independent daily city moves and street positions, with no coordination.
use crate::{
    festival,
    graph::RoadGraph,
    streets::{StreetPoint, distance_m},
};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use thiserror::Error;

/// Bounds the worst-case pair events before allocating trajectories/events.
pub const MAX_PAIR_CHECKS: u64 = 500_000;
const ENCOUNTER_RADIUS_M: f64 = 50.0;
// Keep the city decision stream separate from street sampling.
const STREET_RNG_MASK: u64 = 0x4d45_4554_524f_4144;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Movement {
    pub home_move_probability: f64,
    pub away_move_probability: f64,
    pub home_return_probability: f64,
}
impl Default for Movement {
    fn default() -> Self {
        Self {
            home_move_probability: 0.10,
            away_move_probability: 0.25,
            home_return_probability: 0.60,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EncounterRules {
    pub include_tick_zero: bool,
    pub cross_tick_tolerance: u32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ParticipantCities {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub home_city_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub graduation_city_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SimulationConfig {
    pub agents: u32,
    pub ticks: u32,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub seed: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub same_hometown: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_date: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub participants: Vec<ParticipantCities>,
    pub movement: Movement,
    pub encounters: EncounterRules,
}

#[derive(Debug, Error)]
#[error("invalid simulation parameter {field}: {message}")]
pub struct SimulationError {
    pub field: &'static str,
    pub message: &'static str,
}
fn invalid(field: &'static str, message: &'static str) -> SimulationError {
    SimulationError { field, message }
}

impl SimulationConfig {
    pub fn validate(&self) -> Result<u64, SimulationError> {
        self.validate_with_calendar().map(|(seed, _)| seed)
    }

    fn validate_with_calendar(&self) -> Result<(u64, Option<chrono::NaiveDate>), SimulationError> {
        if !(2..=100).contains(&self.agents) {
            return Err(invalid("agents", "must be between 2 and 100"));
        }
        if !(1..=5300).contains(&self.ticks) {
            return Err(invalid("ticks", "must be between 1 and 5300"));
        }
        if !self.participants.is_empty() && self.participants.len() != self.agents as usize {
            return Err(invalid(
                "participants",
                "must have one entry for each individual",
            ));
        }
        for (field, p) in [
            (
                "movement.home_move_probability",
                self.movement.home_move_probability,
            ),
            (
                "movement.away_move_probability",
                self.movement.away_move_probability,
            ),
            (
                "movement.home_return_probability",
                self.movement.home_return_probability,
            ),
        ] {
            if !p.is_finite() || !(0.0..=1.0).contains(&p) {
                return Err(invalid(field, "must be finite and between 0 and 1"));
            }
        }
        if self.encounters.cross_tick_tolerance != 0 {
            return Err(invalid(
                "encounters.cross_tick_tolerance",
                "only strict same-tick encounters (0) are supported",
            ));
        }
        let checks = u64::from(self.agents) * u64::from(self.agents - 1) / 2
            * (u64::from(self.ticks) + u64::from(self.encounters.include_tick_zero));
        if checks > MAX_PAIR_CHECKS {
            return Err(invalid(
                "ticks",
                "agents × (agents − 1) / 2 × counted ticks must not exceed 500000",
            ));
        }
        if let Some(value) = self.start_date.as_deref() {
            let date = festival::parse_start(value, self.ticks)
                .map_err(|message| invalid("start_date", message))?;
            return Ok((festival::date_seed(date), Some(date)));
        }
        if self.seed.is_empty()
            || self.seed.len() > 20
            || !self.seed.bytes().all(|b| b.is_ascii_digit())
        {
            return Err(invalid("seed", "must be a decimal u64 string"));
        }
        let seed = self
            .seed
            .parse()
            .map_err(|_| invalid("seed", "decimal integer exceeds u64"))?;
        Ok((seed, None))
    }
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Position {
    pub tick: u32,
    pub city_id: String,
    #[serde(flatten)]
    pub street: StreetPoint,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct AgentSummary {
    pub id: u32,
    pub home_city_id: String,
    pub initial_city_id: String,
    pub graduation_city_id: Option<String>,
    pub final_city_id: String,
    pub move_count: u32,
    pub distance_km: f64,
    pub visited_city_ids: Vec<String>,
    pub trajectory: Vec<Position>,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Encounter {
    pub tick: u32,
    pub city_id: String,
    pub street_id: u64,
    pub street_name: Option<String>,
    pub distance_m: f64,
    pub agent_ids: [u32; 2],
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Summary {
    pub any_encounter: bool,
    pub unique_encounter_pairs: usize,
    pub encounter_event_count: usize,
    pub first_encounter_tick: Option<u32>,
    pub all_agents_met: bool,
    pub first_all_agents_tick: Option<u32>,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct SimulationResult {
    pub config: SimulationConfig,
    pub summary: Summary,
    pub agents: Vec<AgentSummary>,
    pub encounters: Vec<Encounter>,
}

fn record_encounters(tick: u32, positions: &[&Position], events: &mut Vec<Encounter>) -> bool {
    let start = events.len();
    for (a, first) in positions.iter().enumerate() {
        for (b, second) in positions.iter().enumerate().skip(a + 1) {
            if first.street.street_id != second.street.street_id {
                continue;
            }
            let separation = distance_m(first.street.coordinates, second.street.coordinates);
            if separation <= ENCOUNTER_RADIUS_M {
                events.push(Encounter {
                    tick,
                    city_id: first.city_id.clone(),
                    street_id: first.street.street_id,
                    street_name: first.street.street_name.clone(),
                    distance_m: separation,
                    agent_ids: [a as u32, b as u32],
                });
            }
        }
    }
    events[start..]
        .sort_unstable_by(|a, b| (&a.city_id, a.agent_ids).cmp(&(&b.city_id, b.agent_ids)));
    events.len() - start == positions.len() * (positions.len() - 1) / 2
}

#[derive(Clone, Copy, Default)]
struct CityChoices {
    home: Option<usize>,
    after_graduation: Option<usize>,
}

fn resolve_city_choices(
    graph: &RoadGraph,
    config: &SimulationConfig,
) -> Result<Vec<CityChoices>, SimulationError> {
    let find = |id: &str, field| {
        graph
            .cities
            .iter()
            .position(|city| city.id == id)
            .ok_or_else(|| invalid(field, "city is not in the active map"))
    };
    let mut choices = vec![CityChoices::default(); config.agents as usize];
    for (choice, input) in choices.iter_mut().zip(&config.participants) {
        choice.home = input
            .home_city_id
            .as_deref()
            .map(|id| find(id, "participants.home_city_id"))
            .transpose()?;
        choice.after_graduation = input
            .graduation_city_id
            .as_deref()
            .map(|id| find(id, "participants.graduation_city_id"))
            .transpose()?;
    }
    if config.same_hometown
        && let Some(home) = choices.iter().find_map(|choice| choice.home)
    {
        for choice in &mut choices {
            if choice.home.is_some_and(|chosen| chosen != home) {
                return Err(invalid(
                    "participants.home_city_id",
                    "shared hometown requires the same city for everyone",
                ));
            }
            choice.home = Some(home);
        }
    }
    Ok(choices)
}

fn destination(
    graph: &RoadGraph,
    current: usize,
    home: usize,
    movement: &Movement,
    rng: &mut ChaCha8Rng,
) -> usize {
    let probability = if current == home {
        movement.home_move_probability
    } else {
        movement.away_move_probability
    };
    if rng.random::<f64>() >= probability {
        return current;
    }
    if current != home && rng.random::<f64>() < movement.home_return_probability {
        return graph
            .step_home(current, home)
            .expect("validated connected road graph");
    }
    let neighbors = graph.neighbors(current);
    let total: f64 = neighbors.iter().map(|(_, d)| 1.0 / d).sum();
    let mut draw = rng.random::<f64>() * total;
    for (city, distance) in &neighbors {
        draw -= 1.0 / distance;
        if draw < 0.0 {
            return *city;
        }
    }
    neighbors
        .last()
        .expect("connected graph has no isolated city")
        .0
}

pub fn simulate(
    graph: &RoadGraph,
    config: &SimulationConfig,
) -> Result<SimulationResult, SimulationError> {
    let (seed, start) = config.validate_with_calendar()?;
    let choices = resolve_city_choices(graph, config)?;
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut street_rng = ChaCha8Rng::seed_from_u64(seed ^ STREET_RNG_MASK);
    let mut homes: Vec<usize> = if config.same_hometown {
        vec![rng.random_range(0..graph.cities.len()); config.agents as usize]
    } else {
        (0..config.agents)
            .map(|_| rng.random_range(0..graph.cities.len()))
            .collect()
    };
    for (home, choice) in homes.iter_mut().zip(&choices) {
        if let Some(chosen) = choice.home {
            *home = chosen;
        }
    }
    let mut current = homes.clone();
    let mut agents: Vec<AgentSummary> = homes
        .iter()
        .zip(&choices)
        .enumerate()
        .map(|(i, (home, choice))| {
            let city = &graph.cities[*home].id;
            AgentSummary {
                id: i as u32,
                home_city_id: city.clone(),
                initial_city_id: city.clone(),
                graduation_city_id: choice
                    .after_graduation
                    .map(|index| graph.cities[index].id.clone()),
                final_city_id: city.clone(),
                move_count: 0,
                distance_km: 0.0,
                visited_city_ids: vec![city.clone()],
                trajectory: Vec::with_capacity(config.ticks as usize + 1),
            }
        })
        .collect();
    let mut events = Vec::new();
    let mut first_all = None;
    for tick in 0..=config.ticks {
        let mut movement = config.movement.clone();
        if let Some(start) = start {
            let date = start + chrono::Days::new(u64::from(tick));
            movement.home_return_probability =
                festival::return_probability(date, movement.home_return_probability);
        }
        for (i, agent) in agents.iter_mut().enumerate() {
            if tick > 0 {
                // On day 1 a chosen graduation destination replaces the usual city decision.
                let relocation = if tick == 1 {
                    choices[i].after_graduation
                } else {
                    None
                };
                let next = relocation.unwrap_or_else(|| {
                    destination(graph, current[i], homes[i], &movement, &mut rng)
                });
                if next != current[i] {
                    agent.move_count += 1;
                    agent.distance_km += if relocation.is_some() {
                        graph.relocation_distance(current[i], next)
                    } else {
                        graph
                            .neighbors(current[i])
                            .into_iter()
                            .find(|(city, _)| *city == next)
                            .expect("one adjacent edge per ordinary move")
                            .1
                    };
                    current[i] = next;
                    agent.final_city_id = graph.cities[next].id.clone();
                    if !agent.visited_city_ids.contains(&agent.final_city_id) {
                        agent.visited_city_ids.push(agent.final_city_id.clone());
                    }
                }
            }
            agent.trajectory.push(Position {
                tick,
                city_id: agent.final_city_id.clone(),
                street: graph.sample_street(current[i], &mut street_rng),
            });
        }
        if tick > 0 || config.encounters.include_tick_zero {
            let positions = agents
                .iter()
                .map(|agent| agent.trajectory.last().expect("daily sample"))
                .collect::<Vec<_>>();
            if record_encounters(tick, &positions, &mut events) && first_all.is_none() {
                first_all = Some(tick);
            }
        }
    }
    let pairs: BTreeSet<_> = events.iter().map(|e| e.agent_ids).collect();
    let summary = Summary {
        any_encounter: !events.is_empty(),
        unique_encounter_pairs: pairs.len(),
        encounter_event_count: events.len(),
        first_encounter_tick: events.first().map(|e| e.tick),
        all_agents_met: first_all.is_some(),
        first_all_agents_tick: first_all,
    };
    Ok(SimulationResult {
        config: config.clone(),
        summary,
        agents,
        encounters: events,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn position(street_id: u64, offset_m: f64, city_id: &str) -> Position {
        Position {
            tick: 1,
            city_id: city_id.into(),
            street: StreetPoint {
                street_id,
                osm_way_id: street_id,
                street_name: None,
                coordinates: [(offset_m / 6_371_008.8).to_degrees(), 0.0],
            },
        }
    }

    #[test]
    fn encounter_requires_the_same_street_and_a_distance_of_at_most_fifty_meters() {
        let first = position(1, 0.0, "a");
        let near = position(1, 49.99, "b");
        let far = position(1, 50.01, "a");
        let other_street = position(2, 0.0, "a");
        let mut events = Vec::new();
        assert!(!record_encounters(
            1,
            &[&first, &near, &far, &other_street],
            &mut events
        ));
        assert_eq!(
            events.iter().map(|e| e.agent_ids).collect::<Vec<_>>(),
            [[0, 1], [1, 2]]
        );
        assert!(
            events
                .iter()
                .all(|e| e.street_id == 1 && e.distance_m <= 50.0)
        );

        events.clear();
        assert!(record_encounters(1, &[&first, &near], &mut events));
        assert_eq!(events[0].city_id, "a");
    }

    #[test]
    fn all_agents_must_be_within_range_of_each_other_for_a_reunion() {
        let points = [
            position(1, 0.0, "a"),
            position(1, 30.0, "a"),
            position(1, 60.0, "a"),
        ];
        let mut events = Vec::new();
        assert!(!record_encounters(
            1,
            &points.iter().collect::<Vec<_>>(),
            &mut events
        ));
        assert_eq!(events.len(), 2);

        let points = [
            position(1, 0.0, "a"),
            position(1, 20.0, "a"),
            position(1, 40.0, "a"),
        ];
        events.clear();
        assert!(record_encounters(
            1,
            &points.iter().collect::<Vec<_>>(),
            &mut events
        ));
        assert_eq!(events.len(), 3);
    }

    #[test]
    fn certain_home_return_takes_shortest_path_and_zero_probability_stays() {
        let graph = RoadGraph::bundled().unwrap();
        let mut rng = ChaCha8Rng::seed_from_u64(1);
        let always = Movement {
            home_move_probability: 1.0,
            away_move_probability: 1.0,
            home_return_probability: 1.0,
        };
        for city in 1..graph.cities.len() {
            assert_eq!(
                destination(&graph, city, 0, &always, &mut rng),
                graph.step_home(city, 0).unwrap()
            );
        }
        let stay = Movement {
            home_move_probability: 0.0,
            away_move_probability: 0.0,
            home_return_probability: 1.0,
        };
        for city in 0..graph.cities.len() {
            assert_eq!(destination(&graph, city, 0, &stay, &mut rng), city);
        }
    }
}
