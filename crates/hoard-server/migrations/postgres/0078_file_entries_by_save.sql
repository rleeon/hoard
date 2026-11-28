-- The account's file entries, readable from an index alone.
--
-- `storage_footprint` (the "free up space" dialog, `/v1/cloud/storage/games`)
-- needs every entry of one account: its id, its save and its content hash. The
-- only index by save is the catalogue key (save_id, relative_path, sha256), which
-- lacks the id, so each entry was a trip to the table. An account's entries are
-- inserted over months between everyone else's, so those trips land on pages
-- scattered across the whole table: on 2026-11-26, 54,823 entries cost 4,550
-- pages of a 678 MB table read from disk, 5.2 s, with 32 MB of shared buffers on
-- the machine. Here they sit together, a few MB read in order.
--
-- Built inline, so new file entries wait while it builds (762k rows that day).
-- Not CONCURRENTLY: that has to run outside the migration's transaction, and it
-- waits for every session holding a snapshot, the ones queued on the migrator's
-- own advisory lock included. A server booting meanwhile, after an OOM restart
-- for instance, would queue there and the two would wait on each other for good;
-- the test harness hung exactly like that. If the lock can't be had in 5 s the
-- migration fails and the deploy with it, and the version serving keeps serving.
SET LOCAL lock_timeout = '5s';
CREATE INDEX IF NOT EXISTS file_entries_by_save_idx
    ON public.file_entries (save_id) INCLUDE (sha256, id);
