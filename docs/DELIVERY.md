# Module delivery evidence

All push operations below target `origin main` in Tokooy/MeetAgain; no environment
configuration was published. Builds/checks execute from repository root.

## Push #1 — 06f646f

`chore: scaffold monorepo (backend/ + frontend/) + Dockerfile skeleton`

Changed files: `.gitignore`, `.dockerignore`, `Cargo.toml`, `Cargo.lock`,
`rust-toolchain.toml`, `backend/Cargo.toml`, `backend/src/main.rs`,
`backend/tests/scaffold.rs`, `frontend/package.json`, `frontend/package-lock.json`,
`frontend/tsconfig.json`, `frontend/index.html`, `frontend/src/main.tsx`,
`frontend/src/styles.css`, `scripts/check.sh`, `Dockerfile`, `README.md`.

Independent Cargo workspace and strict TypeScript React frontend. Honest
construction screen with Main/App entry; styles use system fonts, muted teal,
responsive typography. No fake map, results, or implemented HTTP endpoints.
Cargo can compile without frontend artifacts. Before push: cargo build/test,
fmt, Clippy (-D warnings), frontend tsc + Vite build passed; 1 integration test.
Git add/commit/push completed; main created.

```sh
docker build -t meetagain:module1 .
docker run --rm meetagain:module1
```

This initial skeleton prints version and exits; no web server yet. Docker
verification was completed with the cumulative module 3 image below.

## Push #2 — 471bef0

`feat(backend): load & validate Jiangsu GeoJSON into road graph`

Changed files: `Cargo.lock`, `Dockerfile`, `README.md`, `backend/Cargo.toml`,
`backend/data/SOURCES.md`, `backend/data/jiangsu.geojson`,
`backend/data/roads.json`, `backend/src/graph.rs`, `backend/src/lib.rs`,
`backend/src/main.rs`, `backend/tests/graph.rs`.

GeoJSON structural validation, exact city IDs/names, coordinate/ring checks,
positive geographic weights, no duplicate/self-loop edges, 23-edge requirement,
connectivity and shortest-path home step. Stable city/neighbor ordering.
Records original data bytes, source commit and checksum. Historic generalized
boundaries with undocumented datum; distances approximate, not navigation.

Before push: all checks passed, 5 tests. CLI printed expected 13-node/23-edge
adjacency, every city's home path tested. Git add/commit/push completed.

```sh
docker build -t meetagain:module2 .
docker run --rm meetagain:module2 graph
```

First container build failed at rustup TLS trust, not application code. Module
3 fixes build trust with an optional ephemeral CA secret and avoids a redundant
toolchain sync in the already pinned Rust image. No TLS bypass was used.

## Push #3 — see git log

`feat(backend): agent movement + encounter detection`

Changed files: `Cargo.lock`, `Dockerfile`, `README.md`, `backend/Cargo.toml`,
`backend/data/jiangsu.geojson` (remove executable bit), `backend/src/lib.rs`,
`backend/src/main.rs`, `backend/src/simulation.rs`, `backend/tests/simulation.rs`,
`docs/API.md`, `docs/api.schema.json`, `docs/DELIVERY.md`.

ChaCha8 seeded initialization; synchronous independent movement; home stickiness,
shortest-path return, inverse-distance neighbor choice; exhaustive same-tick
pair events; deduplicated pairs; all-agent reunion as separate statistic;
complete trajectories and CLI JSON; bounded workloads and explicit errors.
API schema/docs are a proposal, not implemented endpoints.

Validation: 13 tests passed (2 movement/event unit tests, 4 graph tests,
1 executable integration test, 6 simulation/CLI tests). cargo build, cargo test,
cargo fmt --check, cargo clippy --all-targets -- -D warnings, frontend
TypeScript --noEmit and Vite build all passed. Encounter tests independently
reconstruct events and trajectory distances. Invalid probabilities (including
NaN/infinity), workload overflow, malformed seeds, zero/too-large agent/tick
counts and cross-tick tolerance rejected.

```sh
docker build --secret id=proxy_ca,src=/etc/ssl/certs/ca-certificates.crt -t meetagain:module3 .
docker run --rm --network none meetagain:module3 graph
docker run --rm --network none meetagain:module3 simulate --agents 20 --ticks 365 --seed 20261116 > result.json
```

Container build completed. Without a corporate/session HTTPS proxy, normal
`docker build -t meetagain:module3 .` needs no secret. Secret CA is not retained
in runtime. Non-root UID/GID 65532, single binary entrypoint, no network needed
for offline execution. Built image size: 76,742,453 bytes.

Representative run: 5207 events, 152 unique pairs, first tick 1; all-agent
reunion false. Host debug and container release result JSON compared equal
byte-for-byte (`cmp`). Output SHA-256:
`0330f8421c7f231e9dc51c46759da3a867023edd676d8ee6cd0f6a33351c0ad6`.
These are finite-run results, not a claim that all humans eventually reunite.

## Next gate

Confirm docs/API.md and docs/api.schema.json before module 4. No HTTP service,
MapLibre interaction, theme switch, chart, or browser end-to-end test has been
implemented or claimed yet. Module 5 supplies the complete frontend; module 6
verifies final single-port HTTP behavior in Docker. The current image is an
offline graph/simulation tool plus construction-screen static assets.
