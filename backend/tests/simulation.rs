mod support;

use geo::{Distance, Haversine, Point};
use meetagain::{
    graph::RoadGraph,
    simulation::{EncounterRules, Movement, ParticipantCities, SimulationConfig, simulate},
};
use std::collections::BTreeSet;
use support::MapFixture;

fn config() -> SimulationConfig {
    SimulationConfig {
        agents: 20,
        ticks: 100,
        seed: "20261116".into(),
        same_hometown: false,
        start_date: None,
        participants: Vec::new(),
        movement: Movement::default(),
        encounters: EncounterRules::default(),
    }
}

#[test]
fn identical_seed_and_config_reproduce_every_output_field() {
    let g = RoadGraph::bundled().unwrap();
    let c = config();
    let a = simulate(&g, &c).unwrap();
    assert_eq!(a, simulate(&g, &c).unwrap());
    let mut other = c;
    other.seed = "20261117".into();
    assert_ne!(a.agents, simulate(&g, &other).unwrap().agents);
}

#[test]
fn every_trajectory_and_encounter_agrees_with_independent_reconstruction() {
    let g = MapFixture::short_streets().graph();
    let c = config();
    let r = simulate(&g, &c).unwrap();
    for a in &r.agents {
        assert_eq!(a.trajectory.len(), c.ticks as usize + 1);
        assert_eq!(a.home_city_id, a.initial_city_id);
        assert_eq!(a.initial_city_id, a.trajectory[0].city_id);
        assert_eq!(a.final_city_id, a.trajectory.last().unwrap().city_id);
        let mut moves = 0;
        let mut distance = 0.0;
        let mut visited = BTreeSet::new();
        for (tick, p) in a.trajectory.iter().enumerate() {
            assert_eq!(p.tick, tick as u32);
            visited.insert(&p.city_id);
        }
        assert_eq!(visited, a.visited_city_ids.iter().collect());
        for pair in a.trajectory.windows(2) {
            if pair[0].city_id != pair[1].city_id {
                let source = g
                    .cities
                    .iter()
                    .position(|c| c.id == pair[0].city_id)
                    .unwrap();
                let target = g
                    .cities
                    .iter()
                    .position(|c| c.id == pair[1].city_id)
                    .unwrap();
                distance += g
                    .neighbors(source)
                    .into_iter()
                    .find(|(n, _)| *n == target)
                    .unwrap()
                    .1;
                moves += 1;
            }
        }
        assert_eq!(a.move_count, moves);
        assert!((a.distance_km - distance).abs() < 1e-9);
    }
    let mut expected = Vec::new();
    let mut pairs = BTreeSet::new();
    let mut first_all = None;
    let pair_limit = r.agents.len() * (r.agents.len() - 1) / 2;
    for tick in 1..=c.ticks as usize {
        let mut daily = Vec::new();
        for a in 0..r.agents.len() {
            for b in a + 1..r.agents.len() {
                let first = &r.agents[a].trajectory[tick];
                let second = &r.agents[b].trajectory[tick];
                let separation = Haversine.distance(
                    Point::from(first.street.coordinates),
                    Point::from(second.street.coordinates),
                );
                if first.street.street_id == second.street.street_id && separation <= 50.0 {
                    daily.push((
                        tick as u32,
                        first.city_id.clone(),
                        [a as u32, b as u32],
                        first.street.street_id,
                        separation,
                    ));
                    pairs.insert([a as u32, b as u32]);
                }
            }
        }
        if first_all.is_none() && daily.len() == pair_limit {
            first_all = Some(tick as u32);
        }
        daily.sort_by(|a, b| (&a.1, a.2).cmp(&(&b.1, b.2)));
        expected.extend(daily);
    }
    assert!(
        !expected.is_empty(),
        "fixture must actually exercise encounters"
    );
    assert_eq!(
        r.encounters
            .iter()
            .map(|e| (
                e.tick,
                e.city_id.clone(),
                e.agent_ids,
                e.street_id,
                e.distance_m
            ))
            .collect::<Vec<_>>(),
        expected
    );
    assert_eq!(r.summary.encounter_event_count, expected.len());
    assert_eq!(r.summary.unique_encounter_pairs, pairs.len());
    assert_eq!(
        r.summary.first_encounter_tick,
        expected.first().map(|e| e.0)
    );
    assert_eq!(r.summary.any_encounter, !expected.is_empty());
    assert_eq!(r.summary.first_all_agents_tick, first_all);
    assert_eq!(r.summary.all_agents_met, first_all.is_some());
}

#[test]
fn zero_city_movement_stays_home_and_tick_zero_is_explicit() {
    let g = MapFixture::short_streets().graph();
    let mut c = config();
    c.agents = 2;
    c.ticks = 5;
    c.same_hometown = true;
    c.movement.home_move_probability = 0.0;
    let r = simulate(&g, &c).unwrap();
    assert!(
        r.agents.iter().all(|a| {
            a.move_count == 0 && a.distance_km == 0.0 && a.visited_city_ids.len() == 1
        })
    );
    assert_eq!(r.summary.first_encounter_tick, Some(1));
    assert_eq!(r.summary.first_all_agents_tick, Some(1));
    assert_eq!(r.encounters.len(), 5);
    c.encounters.include_tick_zero = true;
    let with_initial = simulate(&g, &c).unwrap();
    assert_eq!(with_initial.summary.first_encounter_tick, Some(0));
    assert_eq!(with_initial.summary.first_all_agents_tick, Some(0));
    assert_eq!(&with_initial.encounters[1..], r.encounters.as_slice());
    assert_eq!(r.agents, with_initial.agents);
}

#[test]
fn chosen_hometowns_and_graduation_cities_apply_on_days_zero_and_one() {
    let graph = MapFixture::short_streets().graph();
    let mut c = config();
    c.agents = 2;
    c.ticks = 2;
    c.participants = vec![
        ParticipantCities {
            home_city_id: Some("alpha".into()),
            graduation_city_id: Some("gamma".into()),
        },
        ParticipantCities {
            home_city_id: Some("gamma".into()),
            graduation_city_id: Some("beta".into()),
        },
    ];
    c.movement.home_move_probability = 0.0;
    c.movement.away_move_probability = 0.0;
    let result = simulate(&graph, &c).unwrap();
    let cities = |agent: usize| {
        result.agents[agent]
            .trajectory
            .iter()
            .map(|p| p.city_id.as_str())
            .collect::<Vec<_>>()
    };
    assert_eq!(cities(0), ["alpha", "gamma", "gamma"]);
    assert_eq!(cities(1), ["gamma", "beta", "beta"]);
    assert_eq!(result.agents[0].home_city_id, "alpha");
    assert_eq!(
        result.agents[0].graduation_city_id.as_deref(),
        Some("gamma")
    );
    assert_eq!(result.agents[0].move_count, 1);
    assert_eq!(result.agents[0].visited_city_ids, ["alpha", "gamma"]);
    let planned_km = graph.neighbors(0)[0].1 + graph.neighbors(2)[0].1;
    assert!((result.agents[0].distance_km - planned_km).abs() < 1e-9);
    assert!(result.encounters.is_empty());

    c.same_hometown = true;
    assert_eq!(
        simulate(&graph, &c).unwrap_err().field,
        "participants.home_city_id"
    );
    c.participants[1].home_city_id = None;
    let shared = simulate(&graph, &c).unwrap();
    assert!(shared.agents.iter().all(|a| a.home_city_id == "alpha"));
    c.participants[1].graduation_city_id = Some("missing".into());
    assert_eq!(
        simulate(&graph, &c).unwrap_err().field,
        "participants.graduation_city_id"
    );
}

#[test]
fn invalid_parameters_fail_before_simulation() {
    let mut cases = Vec::new();
    for n in [0, 1, 101, u32::MAX] {
        let mut c = config();
        c.agents = n;
        cases.push(c);
    }
    for ticks in [0, 5301, u32::MAX] {
        let mut c = config();
        c.ticks = ticks;
        cases.push(c);
    }
    for seed in ["", "-1", "1.5", "18446744073709551616", "１２"] {
        let mut c = config();
        c.seed = seed.into();
        cases.push(c);
    }
    for p in [-0.1, 1.1, f64::NAN, f64::INFINITY] {
        let mut c = config();
        c.movement.home_move_probability = p;
        cases.push(c);
        let mut c = config();
        c.movement.away_move_probability = p;
        cases.push(c);
        let mut c = config();
        c.movement.home_return_probability = p;
        cases.push(c);
    }
    let mut c = config();
    c.encounters.cross_tick_tolerance = 1;
    cases.push(c);
    let mut c = config();
    c.participants = vec![ParticipantCities::default()];
    cases.push(c);
    let mut c = config();
    c.agents = 100;
    c.ticks = 1000;
    cases.push(c);
    let g = RoadGraph::bundled().unwrap();
    for c in cases {
        assert!(simulate(&g, &c).is_err(), "accepted invalid config: {c:?}");
    }
    let mut c = config();
    c.seed = u64::MAX.to_string();
    assert!(c.validate().is_ok());
}

#[test]
fn shared_hometown_sets_every_initial_position_and_remains_reproducible() {
    let graph = RoadGraph::bundled().unwrap();
    let mut config = config();
    config.same_hometown = true;
    config.start_date = Some("2027-01-01".into());
    config.seed.clear();
    assert_eq!(config.validate().unwrap(), 20270101);
    let result = simulate(&graph, &config).unwrap();
    let home = &result.agents[0].home_city_id;
    assert!(
        result
            .agents
            .iter()
            .all(|agent| { agent.home_city_id == *home && agent.trajectory[0].city_id == *home })
    );
    assert_eq!(result, simulate(&graph, &config).unwrap());

    config.start_date = None;
    config.seed = "20270101".into();
    let baseline = simulate(&graph, &config).unwrap();
    assert!(
        result
            .agents
            .iter()
            .zip(&baseline.agents)
            .any(|(with, without)| { with.trajectory != without.trajectory })
    );
}

#[test]
fn json_config_rejects_missing_and_unknown_fields() {
    let mut value = serde_json::to_value(config()).unwrap();
    value["agentz"] = 2.into();
    assert!(serde_json::from_value::<SimulationConfig>(value).is_err());
    assert!(serde_json::from_str::<SimulationConfig>("{}").is_err());
}

#[test]
fn cli_outputs_parseable_json_and_fails_on_invalid_agent_count() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_meetagain"))
        .args(["simulate", "--agents", "4", "--ticks", "10", "--seed", "7"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["agents"].as_array().unwrap().len(), 4);
    assert_eq!(
        json["agents"][0]["trajectory"].as_array().unwrap().len(),
        11
    );
    let bad = std::process::Command::new(env!("CARGO_BIN_EXE_meetagain"))
        .args(["simulate", "--agents", "1"])
        .output()
        .unwrap();
    assert!(!bad.status.success());
    assert!(bad.stdout.is_empty());
}
