-- Devices linked to a user. The plan caps how many are active simultaneously.
--
-- A device is created on first login from a given machine + browser/OS combo
-- and last_seen_at is bumped on every authenticated request via the auth
-- middleware. Devices older than 90 days without activity are pruned by the
-- daily cron.

CREATE TABLE IF NOT EXISTS devices (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id       UUID NOT NULL REFERENCES profiles(user_id) ON DELETE CASCADE,
    device_name   TEXT NOT NULL,
    device_kind   TEXT,                  -- 'desktop','steamdeck','mobile',...
    os            TEXT,                  -- 'linux','windows','macos'
    fingerprint   TEXT NOT NULL,         -- hash of stable identifiers
    last_seen_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(user_id, fingerprint)
);

CREATE INDEX IF NOT EXISTS idx_devices_user           ON devices(user_id);
CREATE INDEX IF NOT EXISTS idx_devices_last_seen      ON devices(last_seen_at);
