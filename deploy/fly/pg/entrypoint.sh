#!/usr/bin/env bash
# Supervisor for the cloud machine: Postgres on the volume, hoard-server beside
# it, and the backup schedule.
#
#   serve     Postgres (when /data is a volume), then the server. The server is
#             relaunched whenever it exits (the memory watchdog's bounce, the
#             cutover's restarts); Postgres dying takes the machine down, and
#             Fly restarts it. On SIGINT/SIGTERM the server drains first, then
#             Postgres stops.
#   migrate   Fly's release_command, in a machine of its own without the volume.
#             It reaches this machine's Postgres over the private network
#             through HOARD_MIGRATE_DATABASE_URL, or migrates whatever
#             HOARD__DATABASE__URL names while that is still Supabase.
#   anything else is handed to hoard-server.
set -euo pipefail

PGDATA=${PGDATA:-/data/pg17}
PGBIN=/usr/lib/postgresql/17/bin
SOCKET_DIR=/run/postgresql
RUN_DIR=/run/hoard
CONFIG=/etc/hoard/config.toml
SERVER=/usr/local/bin/hoard-server

log() { echo "entrypoint: $*" >&2; }
# HOME follows the user: inherited from root, every library that looks for a
# dotfile (sqlx's .pgpass, the AWS SDK's config) logs a permission error.
as_postgres() { setpriv --reuid=postgres --regid=postgres --init-groups -- env HOME=/var/lib/postgresql "$@"; }
psql_su() { as_postgres psql -h "$SOCKET_DIR" -U postgres -v ON_ERROR_STOP=1 -q "$@"; }

cmd=${1:-serve}
install -d -m 0755 "$RUN_DIR"
echo $$ >"$RUN_DIR/entrypoint.pid"
case $cmd in
    migrate)
        if [[ -n ${HOARD_MIGRATE_DATABASE_URL:-} ]]; then
            export HOARD__DATABASE__URL=$HOARD_MIGRATE_DATABASE_URL
        fi
        exec setpriv --reuid=hoard --regid=hoard --init-groups -- env HOME=/nonexistent "$SERVER" --config "$CONFIG" migrate
        ;;
    serve) ;;
    *) exec setpriv --reuid=hoard --regid=hoard --init-groups -- env HOME=/nonexistent "$SERVER" --config "$CONFIG" "$@" ;;
esac

# ---- postgres

# A database URL on the local socket with no volume under /data would mean an
# initdb on the machine's root filesystem, which the next deploy wipes along
# with every save anyone made in between. Refuse instead.
have_volume=false
mountpoint -q /data && have_volume=true
if [[ ${HOARD__DATABASE__URL:-} == *"$SOCKET_DIR"* ]] && ! $have_volume && [[ -z ${HOARD_ALLOW_UNMOUNTED_DATA:-} ]]; then
    log "the database URL points at the local socket but /data is not a mounted volume, refusing to start"
    exit 1
fi

backups_on=false
[[ -n ${PGBACKREST_REPO1_S3_BUCKET:-} ]] && backups_on=true

PG_PID=
start_postgres() {
    install -d -o postgres -g postgres -m 0755 "$SOCKET_DIR"
    install -d -o postgres -g postgres -m 0700 "$(dirname "$PGDATA")/pgbackrest" "$PGDATA"
    if [[ ! -s $PGDATA/PG_VERSION ]]; then
        log "empty volume, creating the cluster in $PGDATA"
        # Same collation as the Supabase database it replaces (ICU en-US), and
        # page checksums, which a volume on shared hardware wants.
        as_postgres "$PGBIN/initdb" -D "$PGDATA" -U postgres --data-checksums \
            --encoding=UTF8 --locale=en_US.UTF-8 --locale-provider=icu --icu-locale=en-US \
            --auth-local=peer --auth-host=scram-sha-256 >/dev/null
    fi

    local args=(-D "$PGDATA" -c config_file=/etc/hoard/pg/postgresql.conf)
    if $backups_on; then
        args+=(-c archive_mode=on -c "archive_command=pgbackrest --stanza=hoard archive-push %p")
    fi
    # The release_command machine migrates over the private network. Only
    # when it has somewhere to connect from, and never on a public address.
    if [[ -n ${FLY_PRIVATE_IP:-} ]]; then
        args+=(-c "listen_addresses=localhost,$FLY_PRIVATE_IP")
    fi
    # setpriv execs, so $! is Postgres itself. A shell function here would
    # background a subshell, and signals would stop there.
    setpriv --reuid=postgres --regid=postgres --init-groups -- env HOME=/var/lib/postgresql "$PGBIN/postgres" "${args[@]}" &
    PG_PID=$!

    # Ready means read-write. After a restore Postgres answers read-only while
    # it replays WAL, and the bootstrap below writes; running it then killed
    # the machine in the first restore rehearsal.
    local waited=0
    until as_postgres "$PGBIN/pg_isready" -q -h "$SOCKET_DIR" &&
        [[ $(psql_su -d postgres -Atc 'SELECT pg_is_in_recovery()' 2>/dev/null) == f ]]; do
        if ! kill -0 "$PG_PID" 2>/dev/null; then
            log "postgres exited before accepting connections"
            exit 1
        fi
        # Crash recovery after a hard stop replays WAL before it accepts
        # anything, and on a volume capped at 16 MiB/s that is not instant.
        if ((waited++ > 600)); then
            log "postgres not ready after 5 minutes"
            exit 1
        fi
        sleep 0.5
    done

    psql_su -d postgres <<'SQL'
SELECT 'CREATE ROLE hoard LOGIN' WHERE NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'hoard')
\gexec
SELECT 'CREATE DATABASE hoard OWNER hoard' WHERE NOT EXISTS (SELECT 1 FROM pg_database WHERE datname = 'hoard')
\gexec
SQL
    if [[ -n ${HOARD_PG_PASSWORD:-} ]]; then
        # psql reads it from the environment itself, so the password is
        # neither in anyone's argv nor spliced into SQL by the shell.
        # And never into the log: a failing statement is logged whole, and this
        # one carries the password.
        psql_su -d postgres <<'SQL'
SET log_min_error_statement = panic;
\getenv pw HOARD_PG_PASSWORD
ALTER ROLE hoard PASSWORD :'pw';
SQL
    fi
    psql_su -d hoard -f /etc/hoard/pg/supabase-stub.sql
    psql_su -d hoard <<'SQL'
ALTER SCHEMA auth OWNER TO hoard;
ALTER TABLE auth.users OWNER TO hoard;
ALTER FUNCTION auth.uid() OWNER TO hoard;
CREATE SCHEMA IF NOT EXISTS extensions;
CREATE EXTENSION IF NOT EXISTS pg_stat_statements SCHEMA extensions;
SQL
    if $backups_on; then
        as_postgres pgbackrest --stanza=hoard stanza-create >/dev/null ||
            log "pgbackrest stanza-create failed, backups will not run until it succeeds"
    fi
    log "postgres ready"
}

# ---- backups

# Checked hourly rather than on a clock, so a restart never skips a day: a
# full when the newest full is a week old, a differential when nothing has
# landed in a day. The WAL between them goes up continuously (archive_command).
backup_loop() {
    set +e
    while sleep 3600; do
        local info newest_full newest_any now type=
        if ! info=$(as_postgres pgbackrest --stanza=hoard --output=json info 2>/dev/null); then
            log "backup: pgbackrest info failed"
            continue
        fi
        now=$(date +%s)
        newest_full=$(jq '[.[0].backup[]? | select(.type == "full") | .timestamp.stop] | max // 0' <<<"$info")
        newest_any=$(jq '[.[0].backup[]? | .timestamp.stop] | max // 0' <<<"$info")
        if ((now - newest_full > 7 * 86400)); then
            type="full"
        elif ((now - newest_any > 86400)); then
            type="diff"
        fi
        [[ -z $type ]] && continue
        log "backup: starting $type"
        if as_postgres pgbackrest --stanza=hoard --type="$type" backup; then
            log "backup: $type done"
        else
            log "backup: $type FAILED"
        fi
    done
}

# ---- server

SERVER_PID=
server_started=0
start_server() {
    setpriv --reuid=hoard --regid=hoard --init-groups -- env HOME=/nonexistent "$SERVER" --config "$CONFIG" serve &
    SERVER_PID=$!
    server_started=$(date +%s)
    echo "$SERVER_PID" >"$RUN_DIR/server.pid"
}

stopping=false
shutdown() {
    stopping=true
    log "stopping: server first"
    if [[ -n $SERVER_PID ]] && kill -0 "$SERVER_PID" 2>/dev/null; then
        kill -INT "$SERVER_PID" 2>/dev/null || true
        # The server's own drain is bounded at 20 s; Fly's kill_timeout is 30.
        for _ in $(seq 50); do
            kill -0 "$SERVER_PID" 2>/dev/null || break
            sleep 0.5
        done
        kill -KILL "$SERVER_PID" 2>/dev/null || true
    fi
    if [[ -n $PG_PID ]] && kill -0 "$PG_PID" 2>/dev/null; then
        log "stopping: postgres"
        as_postgres "$PGBIN/pg_ctl" stop -D "$PGDATA" -m fast -t 20 >/dev/null || true
    fi
    exit 0
}
trap shutdown INT TERM

if $have_volume || [[ -n ${HOARD_ALLOW_UNMOUNTED_DATA:-} ]]; then
    start_postgres
    if $backups_on; then
        backup_loop &
    fi
fi

start_server
backoff=1
while true; do
    set +e
    wait -n
    set -e
    $stopping && break
    if [[ -n $PG_PID ]] && ! kill -0 "$PG_PID" 2>/dev/null; then
        log "postgres exited, taking the machine down"
        kill -INT "$SERVER_PID" 2>/dev/null || true
        exit 1
    fi
    if ! kill -0 "$SERVER_PID" 2>/dev/null; then
        # A server that dies within seconds of starting will do so again; back
        # off up to 30 s instead of spinning.
        if (($(date +%s) - server_started < 10)); then
            backoff=$((backoff * 2 > 30 ? 30 : backoff * 2))
        else
            backoff=1
        fi
        log "server exited, relaunching in ${backoff}s"
        sleep "$backoff"
        $stopping && break
        start_server
    fi
done
