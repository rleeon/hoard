-- When the refcount repair first saw a stored object that no version uses.
--
-- Deleting a save used to take its versions with it and leave every blob and
-- chunk at the refcount it had: bytes on disk and in the quota that nothing
-- pointed at and no purge would reach. The repair (`cleanup::repair_refcounts`)
-- finds them by counting the references that exist, but it does not delete on
-- sight. It writes the date here, and only an object still unused a week later
-- goes. NULL = in use, or not looked at yet.
ALTER TABLE blobs ADD COLUMN unreferenced_since TEXT;
ALTER TABLE chunks ADD COLUMN unreferenced_since TEXT;

-- "Does any version still use this sha?" is asked once per object before it is
-- deleted, by the repair and by the purges. Without these it is a scan of every
-- file row the server holds, each time.
CREATE INDEX IF NOT EXISTS idx_snapshot_files_sha ON snapshot_files(sha256);
CREATE INDEX IF NOT EXISTS idx_sfc_chunk ON snapshot_file_chunks(chunk_sha256);
