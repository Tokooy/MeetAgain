use meetagain::{
    graph::RoadGraph,
    simulation::{EncounterRules, Movement, SimulationConfig, simulate},
};
use std::collections::BTreeSet;

fn config() -> SimulationConfig {
    SimulationConfig {
        agents: 20,
        ticks: 100,
        seed: "20261116".into(),
        same_hometown: false,
        start_date: None,
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
    let g = RoadGraph::bundled().unwrap();
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
    for tick in 1..=c.ticks as usize {
        for city in &g.cities {
            for a in 0..r.agents.len() {
                for b in a + 1..r.agents.len() {
                    if r.agents[a].trajectory[tick].city_id == city.id
                        && r.agents[b].trajectory[tick].city_id == city.id
                    {
                        expected.push((tick as u32, city.id.clone(), [a as u32, b as u32]));
                        pairs.insert([a as u32, b as u32]);
                    }
                }
            }
        }
        if first_all.is_none()
            && r.agents
                .iter()
                .all(|a| a.trajectory[tick].city_id == r.agents[0].trajectory[tick].city_id)
        {
            first_all = Some(tick as u32);
        }
    }
    assert!(
        !expected.is_empty(),
        "fixture must actually exercise encounters"
    );
    assert_eq!(
        r.encounters
            .iter()
            .map(|e| (e.tick, e.city_id.clone(), e.agent_ids))
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
fn zero_movement_stays_home_and_tick_zero_is_explicit() {
    let g = RoadGraph::bundled().unwrap();
    let mut c = config();
    c.ticks = 5;
    c.movement.home_move_probability = 0.0;
    let r = simulate(&g, &c).unwrap();
    assert!(
        r.agents
            .iter()
            .all(|a| a.move_count == 0 && a.distance_km == 0.0 && a.visited_city_ids.len() == 1)
    );
    // 20 people in three cities guarantees a pair even without movement.
    assert_eq!(r.summary.first_encounter_tick, Some(1));
    let per_tick = r.encounters.iter().filter(|e| e.tick == 1).count();
    assert_eq!(r.encounters.len(), per_tick * 5);
    c.encounters.include_tick_zero = true;
    let with_initial = simulate(&g, &c).unwrap();
    assert_eq!(with_initial.summary.first_encounter_tick, Some(0));
    assert_eq!(with_initial.encounters.len(), per_tick * 6);
    assert_eq!(
        r.summary.unique_encounter_pairs,
        with_initial.summary.unique_encounter_pairs
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
