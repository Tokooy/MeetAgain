# API contract

[JSON Schemas](api.schema.json) define the request and response shapes under
`$defs`; select the named definition in the table.

| Method | Path | Success | Request | Response schema |
|---|---|---|---|---|
| GET | `/api/health` | 200 | none | HealthResponse |
| GET | `/api/graph` | 200 | none | GraphResponse |
| GET | `/api/map/roads/{city_id}` | 200 | none | StreetNetwork |
| POST | `/api/simulations` | 201 | SimulationRequest | SimulationResponse |
| GET | `/api/simulations/{id}/encounters?cursor=…&limit=100` | 200 | query | EncounterPage |

API routes use JSON and return errors as `ApiError`. The road endpoint serves
GeoJSON. Unknown API paths must
return JSON 404, never the SPA fallback. JSON responses use UTF-8. Methods other
than the listed method return 405. Malformed JSON returns 400, a non-JSON media
type 415, invalid fields/config 422, body over 16 KiB 413, missing/expired result
404. Limit simulation work to one active job; a concurrent request returns 429
with Retry-After. Unexpected internal errors return 500 without implementation
details. A normal POST completes the run before returning (no background job API).

## Explicit request

```json
{
  "agents": 2,
  "ticks": 365,
  "start_date": "2026-11-16",
  "participants": [
    { "home_city_id": "320100", "graduation_city_id": "321100" },
    { "home_city_id": "321000", "graduation_city_id": "320100" }
  ],
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

Unknown properties are rejected. Probabilities are finite in [0,1].
2–100 agents and 1–10,958 simulated days (about 30 years). Each tick represents one simulated day;
tick 0 is the initial placement. Reject workloads where
`agents * (agents - 1) / 2 * (ticks + include_tick_zero)` exceeds 500000.
At 10,958 days, the workload limit allows at most 10 agents. The JSON Schema does
not express the workload check or the numeric upper bound of the seed string.

`participants` is optional. A nonempty array has one entry per agent, in agent
ID order. Its `home_city_id` places the agent in that hometown at tick 0; absent
or null means random. Its `graduation_city_id` sends that agent directly to that
city at tick 1; absent or null uses the ordinary movement decision. These IDs
must be active cities. A graduation move counts as one city change and adds the
shortest city-link distance, even when it spans multiple links.

With `same_hometown: true`, all specified home cities must agree. The single
specified city, or a random city if none is specified, applies to everyone.
Graduation cities remain independent. `start_date` is a
Gregorian `YYYY-MM-DD` date for tick 0. It identifies the trip and seeds the
random generator with the date as a YYYYMMDD integer, taking precedence over any
supplied `seed`. The whole simulation, including the final tick, must fall within
1901–2099. Legacy requests may omit `start_date` or send `null` and supply a
decimal u64 string as `seed` instead. Those runs use the constant baseline and
no workday restrictions.

Tick 1's position is the fixed post-graduation residence. Starting on tick 2,
a simulated workday's sample is in the residence city and strictly less than
15 km from that point, measured as a Haversine distance. If the traveler is
away on a day off, they return by the next workday. The calendar uses published
mainland Chinese holiday and makeup workday schedules for 2026 and 2027. Other
years use Monday through Friday, with lunar New Year days 1–7 off.

The model converts lunar month 1, day 1 to a Gregorian date each year. From 21
days before that date, the return probability rises linearly to
`baseline + 0.75 * (1 - baseline)`, holds from lunar New Year through day 7,
then falls linearly back to baseline over the next seven days. The effect only
changes the direction choice after a traveler away from their hometown decides to move on a day off.
The conversion is checked against the [Hong Kong Observatory calendar](https://www.hko.gov.hk/en/gts/time/conversion.htm).

## Results and event pagination

`SimulationResponse` contains `id`, `graph_version`, echoed `config`, `summary`,
and `agents`. IDs are opaque server-issued strings, separate from the seed;
simulation results and event ordering are reproducible, not HTTP IDs. Agents
numbered from 0. `graduation_city_id` is null when there was no preset move.
`residence_city_id` and `residence_coordinates` are the city and WGS84 point
established on day 1, even if no graduation city was specified.
Each trajectory includes tick 0 and all ticks through the requested duration.
Each tick has the city ID, a point sampled along an OSM road in WGS84
`[longitude, latitude]`, its `osm_way_id`, and its `street_id` and optional
`street_name`. Street positions are sampled by road length each day, including
days with no city change. `visited_city_ids` preserves first-visit order.
`move_count` and `distance_km` describe city changes. Nullable first ticks
use null. No agent reacts to another's position.

A street groups segments of the same OSM way and connected ways that share a
name (or, where unnamed, a route reference). Its ID is the smallest OSM way ID
in the group. Identical names on disconnected ways do not match.

`summary` fields: `any_encounter`, `unique_encounter_pairs`,
`encounter_event_count`, `first_encounter_tick`, `all_agents_met`,
`first_all_agents_tick`. All use the selected tick-zero rule. An encounter is a
pair on the same street at the same tick with a Haversine distance of at most
50 meters, inclusive. The pair counts once per tick, and once overall for the
unique-pair count. A reunion requires every pair to qualify on the same tick.

`EncounterPage`: `simulation_id`, `events`, `next_cursor`. An event contains
`tick`, `city_id`, `street_id`, `street_name` (nullable), `distance_m`, and
`agent_ids: [lower_id, higher_id]`. The city is the lower-ID agent’s city, so a
street crossing a city boundary can record an encounter. Sorted by tick, city ID,
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

City IDs match `^[a-z0-9][a-z0-9_-]{0,31}$`. Center, boundary and road
coordinates use WGS84 `[longitude, latitude]`. Boundary feature properties
contain `city_id` and `name`. Nodes include the OSM boundary relation, extract
URL, replication timestamp (when available), and street segment count.
`graph_version` includes the city list, boundaries, simulation links and
street data. The default map has three cities and three undirected links.
Edge weights are approximate spherical distances between city centers. The
active set comes from `map.json` in the selected map directory.

`GET /api/map/roads/{city_id}` returns a `FeatureCollection` of clipped
OSM `LineString` features as `application/geo+json`, with gzip negotiated
through `Accept-Encoding`. Required feature properties are `osm_id` (way
ID), `highway` (OSM tag), and `class` (`major`, `arterial`, or `local`).
Where present, the import preserves `name`, `ref`, `bridge`, `tunnel`,
and `oneway`. Clipping may produce more than one feature per way. Unknown
or inactive city IDs return JSON 404. See the [README](../README.md#添加城市)
for the city import interface.

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
