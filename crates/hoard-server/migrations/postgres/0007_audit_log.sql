-- Audit log: ops-oriented events (account deletion, plan changes,
-- webhook receipts). Keeps for 1 year.

CREATE TABLE IF NOT EXISTS audit_log (
    id          BIGSERIAL PRIMARY KEY,
    user_id     UUID REFERENCES profiles(user_id) ON DELETE SET NULL,
    actor       TEXT,                        -- 'user','webhook','cron','admin'
    event_type  TEXT NOT NULL,
    entity_id   TEXT,
    metadata    JSONB,
    ip_hash     TEXT,                        -- sha256 of remote ip
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_audit_user_time
    ON audit_log(user_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_audit_event
    ON audit_log(event_type);
