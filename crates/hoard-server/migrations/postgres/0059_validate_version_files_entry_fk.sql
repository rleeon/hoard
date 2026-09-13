-- Second half of 0058: prove the rows already in the table satisfy the
-- constraint it re-created as NOT VALID.
--
-- On its own because of the lock. VALIDATE takes SHARE UPDATE EXCLUSIVE, which
-- lets reads and writes through, where validating inside 0058 would have held
-- that migration's write lock for the whole scan (1.65 s on 703,820 rows).
-- Production had 0 orphaned references when this was written, and the old
-- constraint enforced the same thing up to the moment 0058 dropped it.
SET LOCAL lock_timeout = '5s';

ALTER TABLE public.version_files
    VALIDATE CONSTRAINT version_files_entry_id_fkey;
