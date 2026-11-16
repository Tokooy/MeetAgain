# syntax=docker/dockerfile:1
FROM node:24.22.1-bookworm-slim@sha256:be8204d4bd327363357f532187b752ff62759c31bedf98e9e18fc08c9cbd605c AS frontend-build
WORKDIR /src/frontend
COPY frontend/package.json frontend/package-lock.json ./
RUN npm ci --no-audit --no-fund
COPY frontend/ ./
RUN npm run build

FROM rust:1.90.0-slim-bookworm AS backend-build
WORKDIR /src
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY backend/ backend/
RUN cargo build --release --locked -p meetagain

FROM debian:bookworm-slim AS runtime
WORKDIR /app
COPY --from=backend-build /src/target/release/meetagain /app/meetagain
COPY --from=frontend-build /src/frontend/dist /app/static
USER 65532:65532
EXPOSE 8080
ENTRYPOINT ["/app/meetagain"]
