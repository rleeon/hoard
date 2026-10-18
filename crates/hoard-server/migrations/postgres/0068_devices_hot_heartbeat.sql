-- The heartbeat (every 30 s per device) rewrites last_seen_at, and with two
-- indexes on it no update could be HOT: 364,555 index-touching updates in 39 h
-- on a 510-row table (oct-2026). A scan of 510 rows is free.
-- idx_devices_last_seen_app_version was made by hand in prod, hence IF EXISTS.
DROP INDEX IF EXISTS idx_devices_last_seen;
DROP INDEX IF EXISTS idx_devices_last_seen_app_version;

-- Room in each page for the new row version, which HOT needs.
ALTER TABLE devices SET (fillfactor = 70);
