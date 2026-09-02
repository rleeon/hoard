# syntax=docker/dockerfile:1.7
# Multi-stage build for hoard-server + hoard-admin.
#
# Build:
#   docker build -t hoard:latest -f deploy/docker/Dockerfile .
#
# The build context MUST be the repo root (so the Cargo workspace resolves).

# ---- 1. Builder ------------------------------------------------------------
FROM rust:1.92-slim-bookworm AS builder

# Build deps. libssl-dev for reqwest's native-tls (sqlx uses bundled sqlite,
# so no libsqlite3 needed). pkg-config is required to find openssl.
RUN apt-get update && apt-get install -y --no-install-recommends \
        pkg-config libssl-dev ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /src

# Pre-copy manifests so dependency layers cache when only sources change.
# We can't easily cache deps in a workspace without a complete COPY, so we
# just COPY everything and rely on cargo + BuildKit cache mounts.
COPY . .

# SQLX_OFFLINE=true uses the .sqlx/ cache committed in the repo.
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    SQLX_OFFLINE=true cargo build --release \
        -p hoard-server -p hoard-admin \
 && cp /src/target/release/hoard-server /out-server \
 && cp /src/target/release/hoard-admin  /out-admin

# ---- 2. Runtime ------------------------------------------------------------
FROM debian:bookworm-slim AS runtime

# ca-certificates: outbound TLS; tini: PID 1 reaping; wget: HEALTHCHECK probe.
RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates tini wget \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --system --gid 10001 hoard \
    && useradd --system --uid 10001 --gid 10001 \
               --home-dir /var/lib/hoard --shell /usr/sbin/nologin hoard \
    && mkdir -p /var/lib/hoard/data /var/lib/hoard/tmp /var/lib/hoard/trash \
               /etc/hoard /usr/share/hoard \
    && chown -R hoard:hoard /var/lib/hoard

COPY --from=builder /out-server /usr/local/bin/hoard-server
COPY --from=builder /out-admin  /usr/local/bin/hoard-admin
COPY deploy/docker/entrypoint.sh /usr/local/bin/entrypoint.sh
# NOT under /etc/hoard: that is where operators mount their own config, and the
# mount hides anything the image left there, which is exactly when the example
# is needed. From here an empty mounted directory can still bootstrap itself.
COPY deploy/config.toml.example /usr/share/hoard/config.toml.example
RUN chmod +x /usr/local/bin/entrypoint.sh

# No `USER hoard`: the entrypoint starts as root only to take ownership of the
# mounted directories (a NAS bind mount arrives owned by root) and then re-execs
# itself as PUID:PGID, 10001:10001, the `hoard` user above, unless overridden.
# Passing `--user` / compose `user:` still works and skips that block entirely.
WORKDIR /var/lib/hoard
EXPOSE 12421

# Healthcheck hits the unauthenticated /v1/health route.
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD wget -qO- http://localhost:12421/v1/health >/dev/null 2>&1 || exit 1

ENTRYPOINT ["/usr/bin/tini", "--", "/usr/local/bin/entrypoint.sh"]
CMD ["hoard-server", "--config", "/etc/hoard/config.toml"]
