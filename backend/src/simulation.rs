//! Synchronous independent agents; one edge per simulated day, with no communication.
use crate::graph::RoadGraph;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

/// Bounds the worst-case pair events before allocating trajectories/events.
pub const MAX_PAIR_CHECKS: u64 = 500_000;

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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SimulationConfig {
    pub agents: u32,
    pub ticks: u32,
    pub seed: String,
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
        if !(2..=100).contains(&self.agents) {
            return Err(invalid("agents", "must be between 2 and 100"));
        }
        if !(1..=5300).contains(&self.ticks) {
            return Err(invalid("ticks", "must be between 1 and 5300"));
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
        if self.seed.is_empty()
            || self.seed.len() > 20
            || !self.seed.bytes().all(|b| b.is_ascii_digit())
        {
            return Err(invalid("seed", "must be a decimal u64 string"));
        }
        self.seed
            .parse()
            .map_err(|_| invalid("seed", "decimal integer exceeds u64"))
    }
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Position {
    pub tick: u32,
    pub city_id: String,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct AgentSummary {
    pub id: u32,
    pub home_city_id: String,
    pub initial_city_id: String,
    pub final_city_id: String,
    pub move_count: u32,
    pub distance_km: f64,
    pub visited_city_ids: Vec<String>,
    pub trajectory: Vec<Position>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct Encounter {
    pub tick: u32,
    pub city_id: String,
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

fn record_encounters(
    tick: u32,
    positions: &[usize],
    graph: &RoadGraph,
    events: &mut Vec<Encounter>,
) {
    let mut groups: BTreeMap<usize, Vec<u32>> = BTreeMap::new();
    for (agent, city) in positions.iter().enumerate() {
        groups.entry(*city).or_default().push(agent as u32);
    }
    for (city, agents) in groups {
        for (i, a) in agents.iter().enumerate() {
            for b in &agents[i + 1..] {
                events.push(Encounter {
                    tick,
                    city_id: graph.cities[city].id.clone(),
                    agent_ids: [*a, *b],
                });
            }
        }
    }
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
    let seed = config.validate()?;
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut positions: Vec<usize> = (0..config.agents)
        .map(|_| rng.random_range(0..graph.cities.len()))
        .collect();
    let homes = positions.clone();
    let mut agents: Vec<AgentSummary> = homes
        .iter()
        .enumerate()
        .map(|(i, home)| {
            let city = &graph.cities[*home].id;
            AgentSummary {
                id: i as u32,
                home_city_id: city.clone(),
                initial_city_id: city.clone(),
                final_city_id: city.clone(),
                move_count: 0,
                distance_km: 0.0,
                visited_city_ids: vec![city.clone()],
                trajectory: vec![Position {
                    tick: 0,
                    city_id: city.clone(),
                }],
            }
        })
        .collect();
    let mut events = Vec::new();
    let mut first_all = None;
    for tick in 0..=config.ticks {
        if tick > 0 {
            // Every decision reads the previous tick. No agent reacts to another agent.
            let next: Vec<usize> = positions
                .iter()
                .zip(&homes)
                .map(|(city, home)| destination(graph, *city, *home, &config.movement, &mut rng))
                .collect();
            for (i, city) in next.iter().enumerate() {
                let agent = &mut agents[i];
                if *city != positions[i] {
                    agent.move_count += 1;
                    agent.distance_km += graph
                        .neighbors(positions[i])
                        .into_iter()
                        .find(|(j, _)| j == city)
                        .expect("one adjacent edge per move")
                        .1;
                }
                agent.final_city_id = graph.cities[*city].id.clone();
                if !agent.visited_city_ids.contains(&agent.final_city_id) {
                    agent.visited_city_ids.push(agent.final_city_id.clone());
                }
                agent.trajectory.push(Position {
                    tick,
                    city_id: agent.final_city_id.clone(),
                });
            }
            positions = next;
        }
        if tick > 0 || config.encounters.include_tick_zero {
            record_encounters(tick, &positions, graph, &mut events);
            if first_all.is_none() && positions.iter().all(|p| *p == positions[0]) {
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
    #[test]
    fn three_people_produce_three_pairs_in_stable_order() {
        let graph = RoadGraph::jiangsu().unwrap();
        let mut events = Vec::new();
        record_encounters(4, &[0, 0, 0, 1], &graph, &mut events);
        assert_eq!(
            events.iter().map(|e| e.agent_ids).collect::<Vec<_>>(),
            vec![[0, 1], [0, 2], [1, 2]]
        );
        assert!(events.iter().all(|e| e.tick == 4 && e.city_id == "320100"));
    }
    #[test]
    fn certain_home_return_takes_shortest_path_and_zero_probability_stays() {
        let graph = RoadGraph::jiangsu().unwrap();
        let mut rng = ChaCha8Rng::seed_from_u64(1);
        let always = Movement {
            home_move_probability: 1.0,
            away_move_probability: 1.0,
            home_return_probability: 1.0,
        };
        for city in 1..13 {
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
        for city in 0..13 {
            assert_eq!(destination(&graph, city, 0, &stay, &mut rng), city);
        }
    }
}
