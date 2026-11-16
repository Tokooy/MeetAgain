# API contract — confirmed

Confirmed by the user before module 4. [JSON Schemas](api.schema.json) define all request and
response shapes under `$defs`; select the named definition in the table.

| Method | Path | Success | Request | Response schema |
|---|---|---|---|---|
| GET | `/api/health` | 200 | none | HealthResponse |
| GET | `/api/graph` | 200 | none | GraphResponse |
| POST | `/api/simulations` | 201 | SimulationRequest | SimulationResponse |
| GET | `/api/simulations/{id}/encounters?cursor=…&limit=100` | 200 | query | EncounterPage |

All API routes use JSON, including errors (`ApiError`). Unknown API paths must
return JSON 404, never the SPA fallback. JSON responses use UTF-8. Methods other
than the listed method return 405. Malformed JSON returns 400, a non-JSON media
type 415, invalid fields/config 422, body over 16 KiB 413, missing/expired result
404. Limit simulation work to one active job; a concurrent request returns 429
with Retry-After. Unexpected internal errors return 500 without implementation
details. A normal POST completes the run before returning (no background job API).

## Explicit request

```json
{
  "agents": 20,
  "ticks": 365,
  "seed": "20261116",
  "movement": {
    "home_move_probability": 0.1,
    "away_move_probability": 0.25,
    "home_return_probability": 0.6
  },
  "encounters": {
    "include_tick_zero": false,
    "cross_tick_tolerance": 0
  }
}
```

All fields required; unknown properties rejected. Probabilities finite in [0,1].
2–100 agents, 1–1000 ticks. Seed is decimal u64 encoded as a string; additionally
validate its numeric upper bound (18446744073709551615). Reject workloads where
`agents * (agents - 1) / 2 * (ticks + include_tick_zero)` exceeds 500000.
The JSON Schema does not express these two cross-field/numeric-string checks.

## Results and event pagination

`SimulationResponse` contains `id`, `graph_version`, echoed `config`, `summary`,
and `agents`. IDs are opaque server-issued strings, separate from the seed;
simulation results and event ordering are reproducible, not HTTP IDs. Agents
numbered from 0. All distances in km. Nullable first ticks use null, never -1.
Each trajectory includes initial tick 0 and all ticks through requested `ticks`.
`visited_city_ids` preserves first-visit order. Every move follows exactly one
edge; distances do not create in-transit ticks. No agent sees another's position.

`summary` fields: `any_encounter`, `unique_encounter_pairs`,
`encounter_event_count`, `first_encounter_tick`, `all_agents_met`,
`first_all_agents_tick`. All use the selected tick-zero rule. Repeated same-pair
co-location counts once per tick as an event, once overall as a unique pair.
Three agents together create three pair events, not one group event.

`EncounterPage`: `simulation_id`, `events`, `next_cursor`. An event contains
`tick`, `city_id`, `agent_ids: [lower_id, higher_id]`. Sorted by tick, city ID,
then agent IDs. `limit` defaults to 100; valid integer range 1–1000. Cursor is
an opaque position token bound to that simulation; omit for first page. Invalid
cursor/limit returns 422. End-of-list: `next_cursor: null`. Empty event list is
valid. No event is silently dropped or truncated. API simulation response does
not inline encounters; the offline CLI does.

Results are in-memory only, max 4 runs, oldest creation evicted first, expire
15 minutes after creation. Restart loses all runs; GET on a lost result returns
404. POST response still includes trajectories/summary immediately. UI must
handle expiry and expose download of the result while available.

## Graph

City IDs are six-digit strings. Center and polygon coordinates use
`[longitude, latitude]`. Backend normalizes boundary feature properties to
`city_id` and `name`; frontend joins features to nodes using `city_id`.
`graph_version` identifies the versioned model and checksum of both source data
and road edges. `coordinate_reference` explicitly describes the source-native
frame with undocumented datum; no external basemap is registered to it.
`provenance` contains exact source URL, boundary SHA-256, and the approximate
manual-road model description. Boundaries and edges come only from HTTP, never
from frontend imports of backend files. GET graph has exactly 13 nodes and 23
undirected edges; each edge appears once. Edge weights approximate spherical
center distances, not measured road lengths or travel times.

## Errors

```json
{"error":{"code":"invalid_parameter","message":"must be between 2 and 100","field":"agents"}}
```

`field` is a dotted request path or null. The client uses `code` and `field`, not
English error text, for behavior. No credentials or server internals in errors.

## Single-process serving

Same-origin HTTP on port 8080. `/api/*` reserved for backend; static frontend
assets served by tower-http ServeDir, with SPA fallback only outside `/api/*`.
Backend compiles without frontend dist; runtime static path is configurable.
No runtime template rendering and no second container/process/public port.
