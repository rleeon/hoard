# Local rehearsal of the Fly machine: the image, a volume, an S3 standing in for
# R2, another for pgBackRest (TLS, which it insists on), and a JWKS of our own.
# Nothing here reaches Supabase, R2 or Fly. Sourced, then used step by step:
#
#   source deploy/fly/pg/test/harness.sh
#   infra_up
#   machine_run test1 <env...>
#
# Networks: hoardnet6 carries fdaa:1::/64, so the machine's private address
# matches pg_hba's fdaa::/16 the way Fly's 6PN does. hoardclients is internal:
# a client container on it cannot reach anything outside the rehearsal.

HARNESS_DIR=${HARNESS_DIR:-$HOME/.cache/hoard-fly-rehearsal}
IMAGE=${IMAGE:-hoard-cloud:pg}
MINIO_USER=rehearsal
MINIO_PASS=rehearsal-secret
HERE=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)

infra_up() {
    mkdir -p "$HARNESS_DIR/jwks" "$HARNESS_DIR/tls"
    python3 "$HERE/mint-jwt.py" "$HARNESS_DIR/jwks" init
    if [[ ! -f $HARNESS_DIR/tls/public.crt ]]; then
        openssl req -x509 -newkey rsa:2048 -nodes -days 30 -subj "/CN=minio-pgbr" \
            -addext "subjectAltName=DNS:minio-pgbr" \
            -keyout "$HARNESS_DIR/tls/private.key" -out "$HARNESS_DIR/tls/public.crt" 2>/dev/null
    fi
    docker rm -f jwks minio-r2 minio-pgbr >/dev/null 2>&1 || true
    docker run -d --name jwks --network hoardnet6 -v "$HARNESS_DIR/jwks:/www:ro" \
        busybox:stable httpd -f -p 80 -h /www >/dev/null
    docker run -d --name minio-r2 --network hoardnet6 -e MINIO_ROOT_USER=$MINIO_USER \
        -e MINIO_ROOT_PASSWORD=$MINIO_PASS quay.io/minio/minio:latest server /data >/dev/null
    docker network connect hoardclients minio-r2
    mkdir -p "$HARNESS_DIR/gotrue"
    docker rm -f gotrue >/dev/null 2>&1 || true
    docker run -d --name gotrue --network hoardclients -v "$HARNESS_DIR/gotrue:/tokens:ro" \
        -v "$HERE/fake-gotrue.py:/fake-gotrue.py:ro" python:3.12-alpine python /fake-gotrue.py >/dev/null
    docker run -d --name minio-pgbr --network hoardnet6 -e MINIO_ROOT_USER=$MINIO_USER \
        -e MINIO_ROOT_PASSWORD=$MINIO_PASS -v "$HARNESS_DIR/tls:/root/.minio/certs:ro" \
        quay.io/minio/minio:latest server /data >/dev/null
    sleep 3
    docker run --rm --network hoardnet6 --entrypoint sh quay.io/minio/mc:latest -c "
        mc alias set r2 http://minio-r2:9000 $MINIO_USER $MINIO_PASS >/dev/null &&
        mc mb -p r2/hoard-snapshots-test >/dev/null &&
        mc alias set --insecure pgbr https://minio-pgbr:9000 $MINIO_USER $MINIO_PASS >/dev/null &&
        mc mb --insecure -p pgbr/hoard-pg-backups >/dev/null && echo buckets ready"
}

# The environment a Fly machine would get, pointed at the rehearsal.
machine_env() {
    cat <<EOF
HOARD__SERVER__PUBLIC_URL=http://hoard-api:8082
HOARD__LOGGING__FORMAT=pretty
HOARD__DATABASE__MAX_CONNECTIONS=6
HOARD__CLOUD__SUPABASE_JWKS_URL=http://jwks/jwks.json
HOARD__CLOUD__R2__ENDPOINT=http://minio-r2:9000
HOARD__CLOUD__R2__BUCKET=hoard-snapshots-test
HOARD__CLOUD__R2__ACCESS_KEY_ID=$MINIO_USER
HOARD__CLOUD__R2__SECRET_ACCESS_KEY=$MINIO_PASS
HOARD__CLOUD__MAINTENANCE_FLAG=/data/MAINTENANCE
HOARD__CLOUD__MEMWATCH_TRIP_FRACTION=0.45
MALLOC_ARENA_MAX=2
HOARD_PG_PASSWORD=rehearsal-pg
PGBACKREST_REPO1_S3_BUCKET=hoard-pg-backups
PGBACKREST_REPO1_S3_ENDPOINT=minio-pgbr
PGBACKREST_REPO1_STORAGE_PORT=9000
PGBACKREST_REPO1_STORAGE_VERIFY_TLS=n
PGBACKREST_REPO1_S3_KEY=$MINIO_USER
PGBACKREST_REPO1_S3_KEY_SECRET=$MINIO_PASS
PGBACKREST_REPO1_CIPHER_PASS=rehearsal-cipher
PGBACKREST_REPO1_PATH=/rehearsal
FLY_APP_NAME=hoard-rehearsal
FLY_MACHINE_ID=rehearsal1
FLY_PRIVATE_IP=fdaa:1::10
EOF
}

# machine_run VOLUME MEMORY [extra docker args...] -- runs the image as the Fly
# machine would: SIGINT to stop, 30 s to do it, 1 CPU.
machine_run() {
    local volume=$1 memory=$2
    shift 2
    docker rm -f hoard-api >/dev/null 2>&1 || true
    docker run -d --name hoard-api --hostname hoard-api \
        --network hoardnet6 --ip6 fdaa:1::10 \
        -p 127.0.0.1:18082:8082 \
        --memory "$memory" --memory-swap "$memory" --cpus 1 \
        --stop-signal SIGINT --stop-timeout 35 \
        -v "$volume:/data" \
        --env-file <(machine_env) "$@" "$IMAGE" >/dev/null
    docker network connect hoardclients hoard-api
}

wait_health() {
    local i
    for i in $(seq 240); do
        curl -fsS -o /dev/null http://127.0.0.1:18082/v1/health 2>/dev/null && return 0
        if [[ $(docker inspect -f '{{.State.Running}}' hoard-api 2>/dev/null) != true ]]; then
            echo "hoard-api is not running"; return 1
        fi
        sleep 0.5
    done
    echo "no health after 2 minutes"; return 1
}

token_for() { python3 "$HERE/mint-jwt.py" "$HARNESS_DIR/jwks" token "$1" "$2"; }

# ---- clients

CLIENTS_DIR=${CLIENTS_DIR:-$HARNESS_DIR/clients}

# client_up NAME VERSION TOKEN: one "machine" of an account, a container on the
# internal network with its own HOME and the session file every version since
# 1.0.4 reads when there is no keyring (no D-Bus in here). Supabase points
# nowhere, so Realtime fails and the client falls back to polling, which is
# exactly what it will do once the tables leave Supabase.
client_up() {
    local name=$1 version=$2 token=$3 refresh=${4:-rehearsal-refresh}
    docker rm -f "$name" >/dev/null 2>&1 || true
    docker run -d --name "$name" --hostname "$name" --network hoardclients \
        -e HOME=/home/t -e XDG_CONFIG_HOME=/home/t/.config -e XDG_DATA_HOME=/home/t/.local/share \
        -e XDG_CACHE_HOME=/home/t/.cache -e XDG_RUNTIME_DIR=/home/t/run \
        -e HOARD_CLOUD_URL=http://hoard-api:8082 -e HOARD_SUPABASE_URL=http://gotrue \
        -e HOARD_SUPABASE_ANON_KEY=rehearsal \
        -v "$CLIENTS_DIR/$version/hoard-$version-linux-x86_64:/opt/hoard:ro" \
        hoard-client-test sleep infinity >/dev/null
    docker exec -i "$name" sh -c 'mkdir -p ~/.config/hoard/desktop ~/run ~/saves && chmod 700 ~/run &&
        cat > ~/.config/hoard/desktop/cloud.toml' <<TOML
server_url = "http://hoard-api:8082"

[auth]
access_token = "$token"
refresh_token = "$refresh"
TOML
    if docker exec "$name" test -x /opt/hoard/hoardd; then
        docker exec -d "$name" sh -c '/opt/hoard/hoardd > ~/hoardd.log 2>&1'
        sleep 2
    fi
}

hoard_on() { local name=$1; shift; docker exec "$name" /opt/hoard/hoard "$@"; }

# The resident engine, however this version runs it: hoardd from 1.1.0,
# `hoard sync run` before (the systemd unit's own command, in the foreground).
engine_restart() {
    # By exact name and anchored argv: a bare `pkill -f hoardd` matches this very
    # shell, whose command line contains the word, and kills it first.
    docker exec "$1" sh -c 'pkill -x hoardd; pkill -f "^/opt/hoard/hoard sync run"; sleep 1
        if [ -x /opt/hoard/hoardd ]; then /opt/hoard/hoardd > ~/engine.out 2>&1 &
        else /opt/hoard/hoard sync run > ~/engine.out 2>&1 & fi'
}

db() {
    docker exec hoard-api setpriv --reuid=postgres --regid=postgres --init-groups -- \
        psql -h /run/postgresql -U postgres -d hoard -Atc "$1"
}

engine_logs() { docker exec "$1" sh -c 'cat ~/engine.out ~/.cache/hoard/logs/* 2>/dev/null' | sed 's/\x1b\[[0-9;]*m//g'; }

# One account, two machines on VERSION: A tracks a folder and backs it up, B
# tracks the same game into an empty folder with full sync on and must end up
# with A's bytes, through polling alone. Prints the save id last.
account_flow() {
    local version=$1 tag=$2 user token sid t0
    user=$(cat /proc/sys/kernel/random/uuid)
    token=$(token_for "$user" "$tag-${user:0:8}@test.invalid")
    printf '{"access_token": "%s", "sub": "%s", "email": "%s"}' "$token" "$user" \
        "$tag-${user:0:8}@test.invalid" >"$HARNESS_DIR/gotrue/$user.json"
    client_up "a$tag" "$version" "$token" "$user"
    client_up "b$tag" "$version" "$token" "$user"
    docker exec "a$tag" sh -c 'mkdir -p ~/saves/fac && head -c 300000 /dev/urandom > ~/saves/fac/world.zip'
    sid=$(hoard_on "a$tag" track --slug factorio --path /home/t/saves/fac 2>&1 | sed -n 's/.*save_id: *//p')
    engine_restart "a$tag"
    t0=$(date +%s)
    for _ in $(seq 60); do
        [[ $(db "SELECT latest_version_num FROM saves WHERE id = '$sid'") -ge 1 ]] 2>/dev/null && break
        sleep 3
    done
    echo "$tag: A backed up v$(db "SELECT latest_version_num FROM saves WHERE id = '$sid'") in $(($(date +%s) - t0)) s"
    docker exec "b$tag" sh -c 'mkdir -p ~/saves/fac ~/.local/share/hoard ~/.config/hoard &&
        echo "{\"global_sync\": true, \"auto_restore\": true}" | tee ~/.local/share/hoard/prefs.json > ~/.config/hoard/prefs.json'
    hoard_on "b$tag" track --slug factorio --path /home/t/saves/fac >/dev/null 2>&1
    engine_restart "b$tag"
    t0=$(date +%s)
    # The client holds a download back while the folder looks mid-session
    # (it was just created), four minutes in practice.
    for _ in $(seq 140); do
        docker exec "b$tag" test -f /home/t/saves/fac/world.zip && break
        sleep 3
    done
    local sa sb
    sa=$(docker exec "a$tag" sha256sum /home/t/saves/fac/world.zip | cut -c1-16)
    sb=$(docker exec "b$tag" sh -c 'sha256sum ~/saves/fac/world.zip 2>/dev/null' | cut -c1-16)
    echo "$tag: B restored in $(($(date +%s) - t0)) s, sha A=$sa B=${sb:-missing}"
    echo "$sid"
}

# ---- staging

STAGING_API=https://hoard-server-staging.fly.dev
STAGING_SUPABASE=https://riwbsgfidbqwtnwlenuc.supabase.co
STAGING_ANON=sb_publishable_NSvkfaoYacdmaqwbAr5XKA_DOnwAOyR

# staging_client_up NAME VERSION: a machine signed in to the staging account in
# $STAGING_EMAIL / $STAGING_PASSWORD, with a session of its own (two machines
# sharing one refresh token would trip GoTrue's reuse detection). The password
# only ever lives in the environment of this shell.
staging_client_up() {
    local name=$1 version=$2 session access refresh
    session=$(curl -fsS -X POST "$STAGING_SUPABASE/auth/v1/token?grant_type=password" \
        -H "apikey: $STAGING_ANON" -H "Content-Type: application/json" \
        -d "$(printf '{"email":"%s","password":"%s"}' "$STAGING_EMAIL" "$STAGING_PASSWORD")") || return 1
    access=$(jq -r .access_token <<<"$session")
    refresh=$(jq -r .refresh_token <<<"$session")
    docker rm -f "$name" >/dev/null 2>&1 || true
    docker run -d --name "$name" --hostname "$name" \
        -e HOME=/home/t -e XDG_CONFIG_HOME=/home/t/.config -e XDG_DATA_HOME=/home/t/.local/share \
        -e XDG_CACHE_HOME=/home/t/.cache -e XDG_RUNTIME_DIR=/home/t/run \
        -e HOARD_CLOUD_URL=$STAGING_API -e HOARD_SUPABASE_URL=$STAGING_SUPABASE \
        -e HOARD_SUPABASE_ANON_KEY=$STAGING_ANON \
        -v "$CLIENTS_DIR/$version/hoard-$version-linux-x86_64:/opt/hoard:ro" \
        hoard-client-test sleep infinity >/dev/null
    docker exec -i "$name" sh -c 'mkdir -p ~/.config/hoard/desktop ~/run ~/saves && chmod 700 ~/run &&
        cat > ~/.config/hoard/desktop/cloud.toml' <<TOML
server_url = "$STAGING_API"

[auth]
access_token = "$access"
refresh_token = "$refresh"
TOML
}
