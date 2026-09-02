#!/usr/bin/env bash
# Send an operator broadcast to every Hoard Cloud user's bell panel.
#
# This is THE write path for the `notifications` table, on purpose there is
# no HTTP endpoint that inserts rows, so only someone holding the service-role
# database URL (you) can send one. Delivery is instant for open apps
# (Supabase Realtime pushes the INSERT) and on next poll for the rest.
#
# Usage:
#   DATABASE_URL='postgres://…' tools/send-notification.sh \
#     --title "¡1.000 usuarios!" \
#     --body  "Gracias por confiar en Hoard. **Sois increíbles.**" \
#     [--priority normal|high|low] \
#     [--action-url https://hoard.services/changelog] \
#     [--action-label "Ver novedades"] \
#     [--expires '2026-08-01 00:00:00+00']
#
# Body supports the client's mini-markdown: **bold**, *italic*, `code`,
# [text](https://url), and literal \n for line breaks. Keep it short, the
# bell panel is 288px wide.

set -euo pipefail

TITLE="" BODY="" PRIORITY="normal" ACTION_URL="" ACTION_LABEL="" EXPIRES=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        --title)        TITLE="$2"; shift 2 ;;
        --body)         BODY="$2"; shift 2 ;;
        --priority)     PRIORITY="$2"; shift 2 ;;
        --action-url)   ACTION_URL="$2"; shift 2 ;;
        --action-label) ACTION_LABEL="$2"; shift 2 ;;
        --expires)      EXPIRES="$2"; shift 2 ;;
        *) echo "unknown flag: $1" >&2; exit 2 ;;
    esac
done

[[ -n "$TITLE" && -n "$BODY" ]] || { echo "need --title and --body" >&2; exit 2; }
[[ "$PRIORITY" =~ ^(high|normal|low)$ ]] || { echo "--priority must be high|normal|low" >&2; exit 2; }
[[ -n "${DATABASE_URL:-}" ]] || { echo "set DATABASE_URL to the service-role Postgres URL" >&2; exit 2; }

# psql variable binding (:'var') quotes safely, no SQL injection via the
# message text. Empty optionals become NULL.
psql "$DATABASE_URL" \
    -v ON_ERROR_STOP=1 \
    -v title="$TITLE" \
    -v body="$BODY" \
    -v priority="$PRIORITY" \
    -v action_url="$ACTION_URL" \
    -v action_label="$ACTION_LABEL" \
    -v expires="$EXPIRES" \
    <<'SQL'
INSERT INTO notifications (title, body, priority, action_url, action_label, expires_at)
VALUES (
    :'title',
    :'body',
    :'priority',
    NULLIF(:'action_url', ''),
    NULLIF(:'action_label', ''),
    NULLIF(:'expires', '')::timestamptz
)
RETURNING id, created_at;
SQL

echo "sent: \"$TITLE\" (priority: $PRIORITY)"
