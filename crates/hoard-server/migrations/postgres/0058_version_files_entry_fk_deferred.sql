-- Deleting a cloud save failed with a 500 for every account from 0053 on.
--
-- 0053 split the manifest into a per-save catalogue (`file_entries`) and the
-- rows that point into it (`version_files`), and that drew a diamond: deleting a
-- save cascades down two roads that meet at `version_files`,
--
--   saves -> save_versions -> version_files      (CASCADE, CASCADE)
--   saves -> file_entries  <- version_files      (CASCADE, RESTRICT)
--
-- RESTRICT is checked the moment the catalogue row goes. If the catalogue road
-- runs first, the references are still there and the whole delete aborts; if
-- the versions road runs first, they are gone and it works. Postgres picks the
-- order by trigger name, and the name carries the trigger's OID compared as
-- text. In production the catalogue FK got OID 126023, which sorts before the
-- 17837 of the old versions FK ('2' < '7'), so it always ran first and every
-- delete failed. A fresh database (tests, staging) hands out five-digit OIDs in
-- migration order and gets it right, which is why nothing caught it.
--
-- It took down more than the button. The 30-day account purge and the 7-day
-- archive purge delete through the same cascade: on 2026-09-10 three accounts
-- were past their grace period and still there.
--
-- ## The check moves to commit
--
-- NO ACTION, deferred, instead of RESTRICT. By commit both roads have finished,
-- so the order stops mattering. What 0053 wanted is kept: deleting a catalogue
-- row that a version still points at, on its own, still fails. Only the check's
-- timing changes.
--
-- ## The index the check needs
--
-- Every catalogue row deleted makes Postgres look for references by `entry_id`.
-- The primary key is `(version_id, entry_id)` and cannot answer that, so the
-- lookup read the whole index: 1.77 s per row on 703,820 rows. A save averages
-- 121 catalogue rows and the largest has 35,150, so fixing the order alone
-- would have turned the 500 into a delete that runs for minutes, or for a day.
--
-- NOT VALID here and VALIDATE in 0059: validating scans every row (1.65 s) and
-- doing it in this transaction would hold the write lock on both tables for
-- that long. 0059 validates under a lock that lets writes through. The index
-- build does block writes to `version_files` while it runs; reads do not wait.
--
-- The lock timeout is the guard against the pile-up: a restore reading the
-- manifest holds a lock the DROP has to wait for, and every query behind the
-- DROP waits with it. Better to fail fast and let the migrator retry.
SET LOCAL lock_timeout = '5s';

CREATE INDEX IF NOT EXISTS version_files_entry_id_idx
    ON public.version_files (entry_id);

ALTER TABLE public.version_files
    DROP CONSTRAINT IF EXISTS version_files_entry_id_fkey;
ALTER TABLE public.version_files
    ADD CONSTRAINT version_files_entry_id_fkey
    FOREIGN KEY (entry_id) REFERENCES public.file_entries(id)
    DEFERRABLE INITIALLY DEFERRED
    NOT VALID;
