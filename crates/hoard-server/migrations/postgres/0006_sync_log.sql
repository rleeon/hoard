-- Sync log: append-only audit of cloud sync activity for the user's own
-- /v1/cloud/sync manifest and for quota analytics. Distinct from audit_log
-- (which captures operational events for ops).
--
-- Kind ∈ ('upload','download','restore','delete','quota_block').

CREATE TABLE IF NOT EXISTS sync_log (
    id          BIGSERIAL PRIMARY KEY,
    user_id     UUID NOT NULL REFERENCES profiles(user_id) ON DELETE CASCADE,
    save_id     TEXT REFERENCES saves(id) ON DELETE SET NULL,
    version_num BIGINT,
    kind        TEXT NOT NULL,
    bytes       BIGINT,
    device_id   UUID REFERENCES devices(id) ON DELETE SET NULL,
    metadata    JSONB,
    at          TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_sync_log_user_at
    ON sync_log(user_id, at DESC);
CREATE INDEX IF NOT EXISTS idx_sync_log_save_at
    ON sync_log(save_id, at DESC)
    WHERE save_id IS NOT NULL;
