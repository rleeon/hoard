-- The compression sweep picks blobs in two states: raw, and claimed by a sweep
-- that died before finishing. One OR over both kept the planner off
-- idx_cloud_blobs_raw_created and read the whole table on every sweep: 2.951
-- buffers on a 169k-blob copy, against 203 with the query split in two. A
-- 256 MB machine cannot keep that table in cache, so each sweep would be a full
-- read off a volume capped at 16 MiB/s. This is the index for the second
-- branch, which holds next to nothing.
CREATE INDEX IF NOT EXISTS idx_cloud_blobs_claimed_created
    ON cloud_blobs (created_at)
    WHERE encoding = 'zstd' AND stored_bytes IS NULL;
