-- Save versions: each upload creates a new version row. The blob itself
-- lives in R2 under `r2_key`. The server records size + sha256 for
-- integrity and quota accounting.

CREATE TABLE IF NOT EXISTS save_versions (
    id            BIGSERIAL PRIMARY KEY,
    save_id       TEXT NOT NULL REFERENCES saves(id) ON DELETE CASCADE,
    version_num   BIGINT NOT NULL,
    size_bytes    BIGINT NOT NULL,
    sha256        TEXT NOT NULL,
    r2_key        TEXT NOT NULL,
    device_id     UUID REFERENCES devices(id) ON DELETE SET NULL,
    notes         TEXT,
    is_pinned     BOOLEAN NOT NULL DEFAULT FALSE,
    deleted_at    TIMESTAMPTZ,                 -- soft-delete
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (save_id, version_num)
);

CREATE INDEX IF NOT EXISTS idx_versions_by_save
    ON save_versions(save_id, version_num DESC);
CREATE INDEX IF NOT EXISTS idx_versions_deleted
    ON save_versions(deleted_at)
    WHERE deleted_at IS NOT NULL;
