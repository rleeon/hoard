-- Which versions each machine has run, and when it first showed up on each.
--
-- `devices.app_version` is overwritten by every heartbeat, so it only ever
-- knows the version a machine is on now. Asked how many machines ran 1.1.6,
-- it answered 46: the ones still on it. Everyone who had moved on to 1.2.0 was
-- counted there instead, and the history was gone.
--
-- Keyed by fingerprint, not by device id, and with no foreign key to
-- `devices`: the 90-day prune deletes the device row, and a machine nobody
-- turns on any more still ran the versions it ran. Deleting the account does
-- take the rows with it, through `profiles`.
--
-- `source` says how sure a row is:
--   * seen     written by the trigger below, the moment the version changed.
--   * current  backfill, the version the machine reported last.
--   * log      backfill, a version found in `client_logs` (kept 14 days).
--   * estimate backfill done by hand for releases older than this table.
-- Only `seen` has an exact date. The others carry the earliest evidence.

CREATE TABLE IF NOT EXISTS device_version_history (
    user_id       UUID        NOT NULL REFERENCES profiles(user_id) ON DELETE CASCADE,
    fingerprint   TEXT        NOT NULL,
    app_version   TEXT        NOT NULL,
    first_seen_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    source        TEXT        NOT NULL DEFAULT 'seen'
                              CHECK (source IN ('seen', 'current', 'log', 'estimate')),
    PRIMARY KEY (user_id, fingerprint, app_version)
);

CREATE INDEX IF NOT EXISTS idx_device_version_history_version
    ON device_version_history(app_version, first_seen_at);

-- Same lockdown as every other per-user table (see 0062).
ALTER TABLE device_version_history ENABLE ROW LEVEL SECURITY;

DROP POLICY IF EXISTS device_version_history_self ON device_version_history;
CREATE POLICY device_version_history_self ON device_version_history
    FOR ALL
    USING ((SELECT auth.uid()) = user_id)
    WITH CHECK ((SELECT auth.uid()) = user_id);

-- The trigger rides on the `register_device` upsert, which runs on every
-- heartbeat for every machine. Two things keep that cheap and safe: the WHEN
-- clauses only call the function when a version actually appears or changes,
-- and a failure in here is swallowed. Losing one history row is fine; failing
-- the upsert would fail `/v1/me` and the heartbeat for that machine.
CREATE OR REPLACE FUNCTION record_device_version() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    BEGIN
        INSERT INTO device_version_history (user_id, fingerprint, app_version)
        VALUES (NEW.user_id, NEW.fingerprint, NEW.app_version)
        ON CONFLICT DO NOTHING;
    EXCEPTION WHEN OTHERS THEN
        RAISE WARNING 'device_version_history: %', SQLERRM;
    END;
    RETURN NULL;
END
$$;

DROP TRIGGER IF EXISTS devices_version_history_insert ON devices;
CREATE TRIGGER devices_version_history_insert
    AFTER INSERT ON devices
    FOR EACH ROW
    WHEN (NEW.app_version IS NOT NULL)
    EXECUTE FUNCTION record_device_version();

DROP TRIGGER IF EXISTS devices_version_history_update ON devices;
CREATE TRIGGER devices_version_history_update
    AFTER UPDATE OF app_version ON devices
    FOR EACH ROW
    WHEN (NEW.app_version IS NOT NULL AND NEW.app_version IS DISTINCT FROM OLD.app_version)
    EXECUTE FUNCTION record_device_version();

-- What the database already knows. Re-running this is harmless: a row that
-- exists keeps its date and its source.
INSERT INTO device_version_history (user_id, fingerprint, app_version, first_seen_at, source)
SELECT l.user_id, l.device_fingerprint, l.app_version, min(l.received_at), 'log'
  FROM client_logs l
 WHERE l.device_fingerprint IS NOT NULL
   AND l.app_version IS NOT NULL
   AND EXISTS (SELECT 1 FROM profiles p WHERE p.user_id = l.user_id)
 GROUP BY 1, 2, 3
ON CONFLICT DO NOTHING;

INSERT INTO device_version_history (user_id, fingerprint, app_version, first_seen_at, source)
SELECT d.user_id, d.fingerprint, d.app_version, coalesce(d.last_seen_at, d.created_at), 'current'
  FROM devices d
 WHERE d.app_version IS NOT NULL
ON CONFLICT DO NOTHING;
