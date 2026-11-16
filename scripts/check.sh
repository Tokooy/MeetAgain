#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
cargo fmt --all -- --check
cargo build --locked
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
npm --prefix frontend run build
