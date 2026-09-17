#!/usr/bin/env bash
# Moves the cloud database from Supabase into this machine's Postgres, one step
# at a time, run by hand over `fly ssh console`:
#
#   cutover status   what the server talks to, maintenance, the local copy
#   cutover enter    maintenance on: flag on the volume, server restarted into it
#   cutover copy     dump Supabase, restore here, prove the copy table by table
#   (laptop)         fly secrets set HOARD__DATABASE__URL=<socket URL printed by
#                    copy> HOARD_MIGRATE_DATABASE_URL=<private URL printed by copy>
#                    The machine restarts and comes back still in maintenance,
#                    because the flag lives on the volume.
#   cutover open     maintenance off: flag removed, server restarted
#
# Going back, before `open`: set HOARD__DATABASE__URL to the Supabase URL that
# `copy` kept in /data/cutover/source-url, unset HOARD_MIGRATE_DATABASE_URL,
# then `cutover open`. Nothing was written anywhere while in maintenance, so
# nothing is lost. After `open`, writes land here and going back loses them.
set -euo pipefail

FLAG=/data/MAINTENANCE
WORK=/data/cutover
PG=/etc/hoard/pg
SOCKET=/run/postgresql
API=http://127.0.0.1:8082
LOCAL_URL="postgres:///hoard?host=$SOCKET&user=hoard"

die() { echo "cutover: $*" >&2; exit 1; }
say() { echo "cutover: $*"; }

# What the machine was started with, read off the entrypoint's environment: a
# `fly ssh console` shell does not inherit it, and on Fly PID 1 is Fly's own
# init, whose environment carries none of the secrets (the first staging run
# printed an empty database URL).
machine_env() { tr '\0' '\n' <"/proc/$(cat /run/hoard/entrypoint.pid)/environ" | sed -n "s/^$1=//p"; }
without_password() { sed -E 's#(://[^:/@]+):[^@]*@#\1:***@#'; }
as_hoard() { setpriv --reuid=hoard --regid=hoard --init-groups -- "$@"; }
local_psql() { as_hoard psql "$LOCAL_URL" -v ON_ERROR_STOP=1 "$@"; }

restart_server() {
    local pid
    pid=$(cat /run/hoard/server.pid)
    kill -INT "$pid"
    for _ in $(seq 120); do
        if ! kill -0 "$pid" 2>/dev/null && [[ $(cat /run/hoard/server.pid) != "$pid" ]] &&
            curl -fsS -o /dev/null "$API/v1/health" 2>/dev/null; then
            return 0
        fi
        sleep 0.5
    done
    die "the server did not come back within a minute"
}

in_maintenance() {
    curl -sS "$API/v1/me" 2>/dev/null | jq -e '.code == "maintenance"' >/dev/null 2>&1
}

local_tables() {
    local_psql -At -c "SELECT count(*) FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace
                        WHERE n.nspname IN ('public', 'ops') AND c.relkind IN ('r', 'p')"
}

cmd_status() {
    local url
    url=$(machine_env HOARD__DATABASE__URL)
    say "server database: $(without_password <<<"$url")"
    if [[ -f $FLAG ]]; then say "flag: present"; else say "flag: absent"; fi
    if in_maintenance; then say "server: in maintenance"; else say "server: serving"; fi
    say "local tables: $(local_tables), size: $(local_psql -At -c "SELECT pg_size_pretty(pg_database_size('hoard'))")"
    if [[ -n $(machine_env PGBACKREST_REPO1_S3_BUCKET) ]]; then
        setpriv --reuid=postgres --regid=postgres --init-groups -- pgbackrest --stanza=hoard info | sed 's/^/cutover:   /'
    fi
}

cmd_enter() {
    touch "$FLAG"
    restart_server
    in_maintenance || die "the server restarted but does not answer as in maintenance"
    say "in maintenance. Next: cutover copy"
}

cmd_copy() {
    local src
    src=$(machine_env HOARD__DATABASE__URL)
    [[ $src == *"$SOCKET"* ]] && die "the server already uses the local database"
    if [[ ! -f $FLAG ]] || ! in_maintenance; then
        die "not in maintenance, run cutover enter first"
    fi
    local n
    n=$(local_tables)
    [[ $n == 0 ]] || die "the local database already has $n tables. To copy again: psql -h $SOCKET -U postgres -c 'DROP DATABASE hoard' and restart the machine"

    install -d -m 0700 "$WORK"
    (umask 077 && printf '%s\n' "$src" >"$WORK/source-url")

    say "dumping $(without_password <<<"$src")"
    local t0=$SECONDS
    pg_dump -Fc -n public -n ops -f "$WORK/source.dump" "$src"
    say "dump: $(du -h "$WORK/source.dump" | cut -f1) in $((SECONDS - t0)) s"

    pg_restore -l "$WORK/source.dump" | grep -Ev -f "$PG/restore-exclude.txt" >"$WORK/restore.list"
    t0=$SECONDS
    # As the hoard OS user, for peer auth, so every object is owned by the role
    # the server connects as. Root opens the files and hands them down.
    as_hoard pg_restore -h "$SOCKET" -U hoard -d hoard --no-owner --no-acl --single-transaction \
        -L /dev/fd/3 3<"$WORK/restore.list" <"$WORK/source.dump"
    # As the superuser, or ANALYZE skips the shared catalogs with a warning each.
    setpriv --reuid=postgres --regid=postgres --init-groups -- psql -h "$SOCKET" -U postgres -d hoard -q -c ANALYZE
    say "restore: $((SECONDS - t0)) s"

    say "verifying rows"
    psql "$src" -At -f "$PG/verify-data.sql" >"$WORK/data.source"
    local_psql -At -f "$PG/verify-data.sql" >"$WORK/data.local"
    if ! diff "$WORK/data.source" "$WORK/data.local"; then
        die "row counts or checksums differ (above). Still in maintenance; nothing switched."
    fi
    say "rows: identical in $(wc -l <"$WORK/data.local") tables"

    say "verifying schema"
    { head -n -4 "$PG/schema-fingerprint.sql"
      echo "SELECT kind, name, left(md5(def), 12) FROM objs ORDER BY 1, 2;"; } >"$WORK/schema.sql"
    psql "$src" -At -f "$WORK/schema.sql" >"$WORK/schema.source"
    # Through stdin: $WORK is root's, and psql runs as hoard.
    local_psql -At <"$WORK/schema.sql" >"$WORK/schema.local"
    # Left out of the restore on purpose (restore-exclude.txt): the foreign key
    # to Supabase's auth.users, and a function of Supabase's own.
    grep -Ev '^constraint\|public\.profiles\.profiles_user_id_fkey\||^function\|public\.rls_auto_enable\(\)\|' \
        "$WORK/schema.source" >"$WORK/schema.expected" || true
    if ! diff "$WORK/schema.expected" "$WORK/schema.local"; then
        die "schema differs beyond the expected exclusions (above). Still in maintenance; nothing switched."
    fi
    say "schema: identical but for the expected exclusions"

    local app
    app=$(machine_env FLY_APP_NAME)
    say "copy verified. From the laptop:"
    say "  fly secrets set -a $app \\"
    say "    HOARD__DATABASE__URL='$LOCAL_URL' \\"
    say "    HOARD_MIGRATE_DATABASE_URL='postgres://hoard:<HOARD_PG_PASSWORD>@$(machine_env FLY_MACHINE_ID).vm.$app.internal:5432/hoard'"
    say "then, once it is back: cutover status, and cutover open"
}

cmd_open() {
    local url
    url=$(machine_env HOARD__DATABASE__URL)
    if [[ $url == *"$SOCKET"* ]]; then
        [[ $(local_tables) != 0 ]] || die "the server points at the local database and it is empty"
    else
        say "the server still points at $(without_password <<<"$url"); opening on it"
    fi
    rm -f "$FLAG"
    restart_server
    in_maintenance && die "the server still answers as in maintenance"
    say "open"
}

case ${1:-} in
    status) cmd_status ;;
    enter) cmd_enter ;;
    copy) cmd_copy ;;
    open) cmd_open ;;
    *) die "usage: cutover status|enter|copy|open" ;;
esac
