import type { FeatureCollection, Polygon, MultiPolygon } from "geojson";
export interface City {
  id: string;
  name: string;
  center: [number, number];
}
export interface Road {
  source: string;
  target: string;
  distance_km: number;
}
export interface Graph {
  graph_version: string;
  coordinate_reference: string;
  nodes: City[];
  edges: Road[];
  boundaries: FeatureCollection<
    Polygon | MultiPolygon,
    { city_id: string; name: string }
  >;
  provenance: {
    boundary_source_url: string;
    boundary_sha256: string;
    road_model: string;
  };
}
export interface Config {
  agents: number;
  ticks: number;
  seed?: string;
  same_hometown?: boolean;
  start_date?: string | null;
  movement: {
    home_move_probability: number;
    away_move_probability: number;
    home_return_probability: number;
  };
  encounters: { include_tick_zero: boolean; cross_tick_tolerance: 0 };
}
export interface Position {
  tick: number;
  city_id: string;
}
export interface Agent {
  id: number;
  home_city_id: string;
  initial_city_id: string;
  final_city_id: string;
  move_count: number;
  distance_km: number;
  visited_city_ids: string[];
  trajectory: Position[];
}
export interface Summary {
  any_encounter: boolean;
  unique_encounter_pairs: number;
  encounter_event_count: number;
  first_encounter_tick: number | null;
  all_agents_met: boolean;
  first_all_agents_tick: number | null;
}
export interface Simulation {
  id: string;
  graph_version: string;
  config: Config;
  summary: Summary;
  agents: Agent[];
}
export interface Encounter {
  tick: number;
  city_id: string;
  agent_ids: [number, number];
}
export interface EncounterPage {
  simulation_id: string;
  events: Encounter[];
  next_cursor: string | null;
}
