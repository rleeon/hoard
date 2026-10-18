-- A blob can be stored as a zstd patch against another blob of the same
-- account (encoding 'zstd-patch'). A Fallout 4 save goes from 21 MB to 1-2 MB
-- that way. The base must outlive every patch that needs it: the GC checks
-- this column before it deletes anything.
ALTER TABLE cloud_blobs ADD COLUMN IF NOT EXISTS base_sha256 bytea;

CREATE INDEX IF NOT EXISTS idx_cloud_blobs_base
    ON cloud_blobs (user_id, base_sha256)
    WHERE base_sha256 IS NOT NULL;
