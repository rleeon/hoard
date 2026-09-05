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

ALTER TABLE public.save_version_files
    ALTER COLUMN sha256 TYPE bytea USING decode(sha256, 'hex');

ALTER TABLE public.cloud_blobs
    ALTER COLUMN sha256 TYPE bytea USING decode(sha256, 'hex');
