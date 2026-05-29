-- Client diagnostic logs shipped from connected apps (desktop/CLI).
--
-- Cloud stores only *key events* (INFO+) — the client filters at source per
-- the level advertised in /v1/health, and the ingest handler re-filters as a
-- defense in depth. Rows are pruned after 14 days by the periodic cleanup
-- task spawned in cloud/run.rs.
--
-- `device_id` links to the registered device when one matches by
-- (user_id, fingerprint); the inline metadata columns are always populated so
-- the log row is self-describing even if the device row is later removed.

CREATE TABLE IF NOT EXISTS client_logs (
    id                 UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id            UUID NOT NULL REFERENCES profiles(user_id) ON DELETE CASCADE,
    device_id          UUID REFERENCES devices(id) ON DELETE SET NULL,
    device_name        TEXT,
    device_os          TEXT,
    device_fingerprint TEXT,
    app_version        TEXT,
    level              TEXT NOT NULL,
    target             TEXT,
    message            TEXT NOT NULL,
    fields             JSONB,
    client_ts          TIMESTAMPTZ,
    received_at        TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_client_logs_user_received
    ON client_logs(user_id, received_at DESC);
CREATE INDEX IF NOT EXISTS idx_client_logs_level ON client_logs(level);

-- RLS: same self-ownership pattern as the other user-scoped tables. The
-- service role (the server) bypasses RLS; this is defense in depth for any
-- direct anon-key access.
ALTER TABLE client_logs ENABLE ROW LEVEL SECURITY;
CREATE POLICY client_logs_self ON client_logs
    FOR SELECT USING (auth.uid() = user_id);
