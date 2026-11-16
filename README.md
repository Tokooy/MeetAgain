# MeetAgain · 再相逢

Rust simulation engine and an independent React frontend in one repository.
Work in the existing checkout; no additional worktree is needed.

## Status

Modules 1–2: scaffold and validated city road graph. The executable prints its version or graph and exits;
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
