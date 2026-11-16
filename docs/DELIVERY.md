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

## API gate history

The user confirmed the final API contract before module 4. The following
sections record implementation and final container checks after confirmation.

## Push #4 — ca548e0

`feat(backend): axum HTTP API for graph simulation and encounters`

Files: Cargo.lock, backend/Cargo.toml, backend/src/{api,lib,main}.rs,
backend/tests/api.rs, Dockerfile, docs/API.md.

Confirmed JSON contract implemented. Bounded blocking simulation worker,
JSON error handling, request limits, four-run FIFO cache with 15-minute TTL,
simulation-bound pagination cursors, normalized map features, same-origin
ServeDir and SPA routing. JSON float parsing supports exact round trips.
Cargo builds without frontend artifacts; `serve --static-dir` is a runtime option.

Checks before push passed: cargo build/test (19 tests), fmt, Clippy -D warnings,
TypeScript and Vite build. API tests cover engine equivalence, pagination,
request errors, eviction, TTL, concurrency, static routing and API/SPA isolation.

```sh
docker build --secret id=proxy_ca,src=/etc/ssl/certs/ca-certificates.crt -t meetagain:module4 .
docker run --rm -p 8080:8080 meetagain:module4
```

## Push #5 — see git log

`feat(frontend): modern map controls playback and encounter visualization`

Files: .gitignore, frontend/package{,-lock}.json, frontend/vite.config.ts,
frontend/playwright.config.ts, frontend/e2e/app.spec.ts,
frontend/src/{main,App}.tsx, frontend/src/{api,store,types}.ts,
frontend/src/vite-env.d.ts, frontend/src/styles.css,
frontend/src/components/{CityMap,Controls,PanelSection,Playback,Results}.tsx,
docs/DELIVERY.md, docs/screenshots/{desktop-dark,desktop-light,mobile}.png.

React 19, strict TypeScript, Vite, Tailwind v4, MapLibre GL v6, Recharts v3,
Zustand and TanStack Query. MapLibre's module worker is explicitly bundled by
Vite; no third-party runtime scripts, tiles, glyphs, fonts or API keys needed.
Dark/light themes, collapsed mobile panels, 200ms movement (reduced motion
respected), time scrubber, selected-agent fading route, event filtering,
full trajectory viewer and JSON downloads. All data comes from HTTP APIs.

Component structure: App orchestrates TanStack requests, Controls edits explicit
model inputs, CityMap displays GeoJSON and positions, Playback updates tick,
Results displays statistics/chart/trajectory/events. PanelSection controls
responsive collapsible panels. Zustand stores only local viewing state.

Screenshots: dark teal map at center, restrained city labels and dashed road
links, controls left, results right; light palette uses a pale map and deep teal
accents; narrow viewport stacks collapsible panels around a full-width map.
Images in docs/screenshots are actual Chromium captures, not mockups.

Verification: all required Rust and frontend checks before push; 3 Playwright
flows passed with actual backend, including rendered map feature readiness,
nonzero canvas dimensions, no browser runtime exceptions, seed-20261116 results,
playback, filter, full 5207-event download, reset, themes, reduced-motion mobile,
no desktop vertical/mobile horizontal overflow, workload validation and a 429
response. Axe WCAG2 A/AA and WCAG2.1 AA checks reported no violations on tested
light desktop and dark mobile states. These are automated checks, not a claim
of exhaustive accessibility certification.

```sh
docker build --secret id=proxy_ca,src=/etc/ssl/certs/ca-certificates.crt -t meetagain:module5 .
docker run --rm -p 8080:8080 meetagain:module5
```

## Push #6 — see git log

`chore: document workflow and verify final single-process Docker application`

Files: .dockerignore, .node-version, README.md, backend/src/api.rs,
backend/tests/api.rs, frontend/package{,-lock}.json,
frontend/src/{App.tsx,api.ts}, frontend/e2e/api.spec.ts,
docs/DELIVERY.md, docs/screenshots/{desktop-dark,desktop-light,mobile}.png.

Final documentation, pinned local Node version, AbortSignal-aware frontend
requests, SIGTERM/SIGINT graceful shutdown, JSON error envelope for invalid
UTF-8 path IDs, Ajv contract validation on real HTTP responses, exclusion of
browser test artifacts from Docker context. Actual final-container screenshots
replace intermediate captures. No environment configuration was published.

Checks before push:

- cargo build/test passed: 20 tests, none failed/skipped; fmt and Clippy
  `--all-targets -- -D warnings` passed.
- TypeScript strict `tsc --noEmit`, Vite production build and frontend Prettier
  check passed. Vite reports the expected lazy MapLibre vendor chunk >500 KiB
  and ignored third-party `use client` directives; these are build warnings,
  not failed checks. MapLibre JS and module worker are served locally.
- Three-stage final Docker build passed, using the optional CA secret with TLS
  verification preserved. One temporary crates download proxy failure cleared
  after a successful destination check and rebuild.
- Four Playwright tests passed against the final Docker image, including Ajv
  validation of GraphResponse, HealthResponse, SimulationResponse, EncounterPage
  and ApiError, plus the three browser flows from module 5. Axe found no WCAG
  A/AA violations in the tested states. No fixture API is used for the main
  simulation; only the explicit 429 error scenario intercepts a request.
- Runtime verified with non-root UID/GID 65532, read-only root filesystem,
  dropped capabilities and no-new-privileges. One application process
  (`meetagain`, PID 1), one exposed/published port (8080); proc inspection adds
  only the temporary inspection shell. Docker top was unavailable due to this
  managed runtime's cgroup enumeration, so process verification used /proc.
- Final image size 82,530,973 bytes. Offline release JSON still matches host
  debug JSON byte-for-byte. SHA-256:
  `0330f8421c7f231e9dc51c46759da3a867023edd676d8ee6cd0f6a33351c0ad6`.
- Docker stop completed within its 5-second timeout with exit code 0. Restart
  restored healthy HTTP/static service; old in-memory run returned JSON 404,
  matching the documented cache lifecycle.

```sh
docker build --platform=linux/amd64 \
  --secret id=proxy_ca,src=/etc/ssl/certs/ca-certificates.crt \
  -t meetagain:latest .
docker run --rm --name meetagain -p 8080:8080 meetagain:latest
```

For local development, build frontend then `cargo run --locked -- serve
--address 127.0.0.1:8080 --static-dir frontend/dist`. Use `bash scripts/check.sh`,
`npm --prefix frontend run format:check`, then (with service running)
`npm --prefix frontend run test:e2e`.

All six modules are implemented. Geographic data limitations remain those
explicitly documented in backend/data/SOURCES.md and README; no current official
boundary accuracy, real navigation routing or empirical human behavior is
claimed. No new-task snapshot restoration or environment publication is claimed.
