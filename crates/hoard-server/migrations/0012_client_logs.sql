-- Client diagnostic logs shipped from connected apps (desktop/CLI).
--
-- Self-hosted stores *everything* the client sends (down to DEBUG); the
-- server advertises its accepted minimum level via /v1/health and the
-- client filters at source. Rows are pruned after 14 days by the hourly
-- cleanup task (see cleanup.rs).
--
-- The device is identified by hostname + a stable fingerprint. Self-hosted
-- has no `devices` table, so we just store the metadata inline.

CREATE TABLE IF NOT EXISTS client_logs (
    id                 TEXT PRIMARY KEY NOT NULL,   -- UUID v4 as text
    user_id            TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    device_id          TEXT,                        -- nullable; unused self-hosted
    device_name        TEXT,
    device_os          TEXT,
    device_fingerprint TEXT,
    app_version        TEXT,
    level              TEXT NOT NULL,               -- 'trace'|'debug'|'info'|'warn'|'error'
    target             TEXT,
    message            TEXT NOT NULL,
    fields             TEXT,                        -- JSON object as text
    client_ts          TEXT,                        -- RFC3339, as reported by client
    received_at        TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_client_logs_user_received
    ON client_logs(user_id, received_at DESC);
CREATE INDEX IF NOT EXISTS idx_client_logs_level ON client_logs(level);
