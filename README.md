# MeetAgain · 再相逢

Rust simulation engine and an independent React frontend in one repository.
Work in the existing checkout; no additional worktree is needed.

## Status

Modules 1–3: scaffold, validated city road graph and offline simulation. The executable prints its version or graph and exits;
there is no HTTP service yet. The frontend is an honest construction screen.

## Development

Rust 1.90.0 (rustup reads `rust-toolchain.toml`), Node 24.22.1, npm 11.

```sh
npm --prefix frontend ci
bash scripts/check.sh
cargo run --locked -p meetagain
```

## Container skeleton

```sh
docker build -t meetagain:module1 .
docker run --rm meetagain:module1
```

The three stages build frontend, compile backend, and copy only binary/static
assets to the runtime. One backend process will serve both API and static files
on port 8080 after the API module. No frontend artifacts are required by Cargo.

## Delivery order

1. Scaffold and Docker skeleton.
2. Validated Jiangsu GeoJSON and curated road graph.
3. Seeded agent movement and encounter detection.
4. API, after explicit contract confirmation.
5. Interactive map and results.
6. Documentation and end-to-end Docker verification.

## Graph inspection

```sh
cargo run --locked -p meetagain -- graph
docker build -t meetagain:module2 .
docker run --rm meetagain:module2 graph
```

13 cities, 23 manual undirected edges; weights approximate center-to-center
spherical distances. See [data provenance](backend/data/SOURCES.md) for the
exact pinned source, historical boundary and coordinate limitations. No
runtime download or credentials are required.

## Offline simulation (module 3)

```sh
cargo run --locked -p meetagain -- simulate --agents 20 --ticks 365 --seed 20261116 > result.json
docker build -t meetagain:module3 .
docker run --rm --network none meetagain:module3 simulate --agents 20 --ticks 365 --seed 20261116 > result.json
```

For this managed cloud's HTTPS proxy, keep TLS verification and mount the CA
only as a build secret (not copied into the image):

```sh
docker build --secret id=proxy_ca,src=/etc/ssl/certs/ca-certificates.crt -t meetagain:module3 .
```

Images and the Dockerfile frontend are pinned by digest; current runtime pin
is linux/amd64. Cargo/npm use checked-in lockfiles. Image Rust is already the
pinned version, so the local rust-toolchain.toml is not copied into its build
stage (avoids downloading local-only fmt/clippy components). Non-root runtime
contains only binary and built frontend over the Debian base.

Default behavior: home is the randomly assigned starting city. Each tick,
independent agents move with probability 0.10 at home / 0.25 away. On deciding
to move while away, probability 0.60 chooses one step on a distance-shortest
route home; otherwise choose a neighbor proportional to inverse edge distance.
Decisions are synchronous; no communication or pursuit. CLI flags `--home-move`,
`--away-move`, `--home-return` expose these probabilities. `--include-tick-zero`
explicitly counts initial co-location; default excludes tick 0. No cross-tick
or edge-crossing encounters. Same-city pairs are recorded once per tick.

ChaCha8 with a fixed decimal u64 seed, stable city/neighbor ordering and the
locked dependency versions reproduces a run. Agent IDs start at 0. All tick
positions, visited cities, move count, approximate distance and every pair
event are in the JSON. Distinct pairs and first tick are summarized; whether
ALL agents met simultaneously is a separate statistic. Event order is tick,
city ID, then agent IDs. Probabilities may be 0 or 1; finite-run interpretation
still applies. At most 100 agents, 1000 ticks and 500000 worst-case pair checks
(including tick 0 only when enabled); invalid/over-budget runs fail explicitly.

A finite run can demonstrate an encounter, not establish that reunion is
impossible forever. With the default positive lazy movement on this connected
finite graph, the idealized unlimited independent Markov model eventually
reaches reunion states with probability 1. This is not a promise about people,
finite horizons, or parameter choices that make movement non-ergodic.

### Verified example

20 agents, 365 ticks, seed `20261116`, default probabilities:
5207 pair events, 152 unique pairs, first encounter tick 1, no all-agent reunion.
Host debug output and release output in the no-network container are compared
byte-for-byte during delivery. See [delivery evidence](docs/DELIVERY.md).

HTTP and interactive frontend modules await [API contract](docs/API.md)
confirmation. Current frontend is still a construction screen.
