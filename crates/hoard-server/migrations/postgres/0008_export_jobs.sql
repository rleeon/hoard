-- Export jobs: long-running tasks that produce a downloadable ZIP of all
-- the user's saves. The cron sweeps finished jobs older than 7 days.

CREATE TABLE IF NOT EXISTS export_jobs (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id      UUID NOT NULL REFERENCES profiles(user_id) ON DELETE CASCADE,
    status       TEXT NOT NULL DEFAULT 'pending'
                     CHECK (status IN ('pending','running','done','failed','expired')),
    r2_key       TEXT,                   -- where the ZIP lives (once done)
    size_bytes   BIGINT,
    error        TEXT,
    requested_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    started_at   TIMESTAMPTZ,
    finished_at  TIMESTAMPTZ,
    expires_at   TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_export_jobs_user_time
    ON export_jobs(user_id, requested_at DESC);
CREATE INDEX IF NOT EXISTS idx_export_jobs_status
    ON export_jobs(status);
