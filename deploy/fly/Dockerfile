# syntax=docker/dockerfile:1.7
# Multi-stage build for hoard-server in CLOUD mode (Fly.io), with the cloud
# database's Postgres in the same image (see deploy/fly/pg/).
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
# Debian rather than distroless now that Postgres lives here: it needs a shell
# to be supervised, and the same major as Supabase (17) from the PostgreSQL
# project's own repository. Bookworm to match the builder's glibc.
FROM debian:bookworm-slim AS runtime

# postgresql-common creates a cluster on install unless told not to; ours is
# made on the volume at first boot, never in the image.
RUN mkdir -p /etc/postgresql-common \
 && echo 'create_main_cluster = false' > /etc/postgresql-common/createcluster.conf \
 && apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates curl tini jq procps locales \
 && echo 'en_US.UTF-8 UTF-8' > /etc/locale.gen && locale-gen \
 && install -d /usr/share/postgresql-common/pgdg \
 && curl -fsSL -o /usr/share/postgresql-common/pgdg/apt.postgresql.org.asc \
        https://www.postgresql.org/media/keys/ACCC4CF8.asc \
 && echo 'deb [signed-by=/usr/share/postgresql-common/pgdg/apt.postgresql.org.asc] https://apt.postgresql.org/pub/repos/apt bookworm-pgdg main' \
        > /etc/apt/sources.list.d/pgdg.list \
 && apt-get update && apt-get install -y --no-install-recommends \
        postgresql-17 postgresql-client-17 pgbackrest \
 && rm -rf /var/lib/apt/lists/* \
 && useradd --system --uid 10001 --home-dir /nonexistent --shell /usr/sbin/nologin hoard

COPY --from=builder /out-server /usr/local/bin/hoard-server
COPY deploy/config.cloud.toml.example /etc/hoard/config.toml
COPY deploy/fly/pg/postgresql.conf deploy/fly/pg/pg_hba.conf deploy/fly/pg/supabase-stub.sql \
     deploy/fly/pg/verify-data.sql deploy/fly/pg/schema-fingerprint.sql \
     deploy/fly/pg/restore-exclude.txt /etc/hoard/pg/
COPY deploy/fly/pg/pgbackrest.conf /etc/pgbackrest/pgbackrest.conf
COPY --chmod=0755 deploy/fly/pg/entrypoint.sh /usr/local/bin/entrypoint.sh
COPY --chmod=0755 deploy/fly/pg/cutover.sh /usr/local/bin/cutover

EXPOSE 8082

# tini reaps and forwards the stop signal to the entrypoint alone, which stops
# the server before Postgres. `-s` because on Fly it is not PID 1 (Fly's init
# is) and has to register as the reaper for the processes below it.
# `serve` goes in CMD, not ENTRYPOINT: Fly's release_command replaces CMD and
# keeps ENTRYPOINT, so with `serve` baked in, the migrate step ran as
# `... serve migrate` and clap rejected the extra argument.
ENTRYPOINT ["/usr/bin/tini", "-s", "--", "/usr/local/bin/entrypoint.sh"]
CMD ["serve"]
