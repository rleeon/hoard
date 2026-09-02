# syntax=docker/dockerfile:1.7
# Multi-stage build for hoard-server in CLOUD mode (Fly.io).
# Builds with `--features cloud` (Postgres + Supabase JWT + R2 + Lemon Squeezy).
#
# Build:
#   docker build -t hoard-cloud:latest -f deploy/fly/Dockerfile .
#
# Context MUST be the repo root.

# ---- 1. Builder ------------------------------------------------------------
FROM rust:1.92-slim-bookworm AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
        pkg-config libssl-dev ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /src
COPY . .

# We use sqlx runtime queries in cloud code (no .sqlx/ cache needed for the
# cloud module). Self-hosted code still uses the macro against SQLite,
# SQLX_OFFLINE points at the committed cache for that.
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    SQLX_OFFLINE=true cargo build --release \
        --features cloud -p hoard-server \
 && cp /src/target/release/hoard-server /out-server

# ---- 2. Runtime ------------------------------------------------------------
# Distroless gives us libc + ca-certificates and not much else. Smallest
# attack surface and ~25 MB final image.
FROM gcr.io/distroless/cc-debian12:nonroot AS runtime

COPY --from=builder /out-server /usr/local/bin/hoard-server
COPY deploy/config.cloud.toml.example /etc/hoard/config.toml

EXPOSE 8082

# distroless has no shell; ENTRYPOINT is the binary directly. Fly's
# healthcheck (configured in fly.toml) probes /v1/health over HTTP.
# `serve` goes in CMD, not ENTRYPOINT: Fly's release_command replaces CMD and
# keeps ENTRYPOINT, so with `serve` baked in, the migrate step ran as
# `... serve migrate` and clap rejected the extra argument.
ENTRYPOINT ["/usr/local/bin/hoard-server", "--config", "/etc/hoard/config.toml"]
CMD ["serve"]
