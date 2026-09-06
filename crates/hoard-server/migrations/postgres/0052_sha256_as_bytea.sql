-- A SHA-256 is 32 bytes. Stored as lowercase hex in a `text` column it takes
-- 65 (64 characters plus the varlena header), so more than half of every sha in
-- the database is the encoding rather than the digest.
--
-- It is paid twice on `cloud_blobs`, whose primary key is `(user_id, sha256)`,
-- and once per manifest row on `save_version_files`, of which there are 623k.
-- Measured before writing this: ~21 MB off the manifest table, ~4 MB off the
-- blob table and ~5 MB off its primary key.
--
-- Both columns convert together and cannot be split. Three queries join them
-- directly (`archive.rs`, in the frozen/archivable/shared-group sizing), and
-- with one side bytea and the other text Postgres refuses with
-- `operator does not exist: bytea = text`.
--
-- Checked against production first: 623,713 manifest rows and 129,019 blob
-- rows, every one of them matching `^[0-9a-f]{64}$`, so the USING clause below
-- cannot fail on a stray value.
--
--   SELECT count(*) FROM save_version_files WHERE sha256 !~ '^[0-9a-f]{64}$';
--
-- `save_versions.sha256` is deliberately NOT converted. It is a different
-- column with a different meaning: the digest of a whole-archive version, and
-- the empty string is its sentinel for "not applicable" on content-addressed
-- ones. That sentinel has no clean bytea spelling, the table is small, and
-- several queries test `sha256 = ''` to find uploads that never committed.
--
-- The Rust side keeps passing and receiving lowercase hex. Nothing about the
-- wire changes; the queries gained `decode($n, 'hex')` on the way in and
-- `encode(col, 'hex')` on the way out. That was chosen over converting in Rust
-- because the cloud module uses runtime `sqlx::query()`: a `Vec<u8>` bound
-- where a `String` belongs compiles fine and only fails in front of a user,
-- whereas a missing `decode()` is visible in the diff.
--
-- This one rewrites both tables, so it holds an ACCESS EXCLUSIVE lock for as
-- long as that takes and needs transient room roughly the size of each table.
-- Unlike 0050 it cannot be split into "deploy the code, then migrate": the old
-- binary cannot read bytea and the new one cannot read text, so the code and
-- the schema have to land together.

-- The primary key comes off first and goes back on after. Not for correctness:
-- for room. `ALTER COLUMN TYPE` writes a new copy of the table *and* of every
-- index on it before dropping the old, and on the free plan (500 MB) the peak
-- with the 77 MB key in place is ~534 MB. Dropping it first puts the peak at
-- ~374 MB, which fits with margin.
--
-- The window between the two statements is why this migration needs the app
-- stopped: without that key there is no `ON CONFLICT (save_id, version_num,
-- relative_path)` for the manifest insert to land on, so any upload arriving
-- mid-migration would error. Stop the machine, deploy (the release command runs
-- the migration), let it come back up.
ALTER TABLE public.save_version_files
    DROP CONSTRAINT save_version_files_pkey;

ALTER TABLE public.save_version_files
    ALTER COLUMN sha256 TYPE bytea USING decode(sha256, 'hex');

ALTER TABLE public.save_version_files
    ADD CONSTRAINT save_version_files_pkey PRIMARY KEY (save_id, version_num, relative_path);

-- Small enough (34 MB with its key) that it needs none of the above.
ALTER TABLE public.cloud_blobs
    ALTER COLUMN sha256 TYPE bytea USING decode(sha256, 'hex');
