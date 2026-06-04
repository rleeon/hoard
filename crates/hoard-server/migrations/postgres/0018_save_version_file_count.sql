-- File count per cloud save version. Self-hosted (SQLite) snapshots already
-- record this, but the cloud path uploads an opaque tar.zst and never told the
-- server how many files it held, so the History view rendered "0 archivos"
-- next to a non-zero size. DEFAULT 0 keeps pre-existing rows valid (their true
-- count is unknown and not worth backfilling from R2).
ALTER TABLE save_versions ADD COLUMN file_count BIGINT NOT NULL DEFAULT 0;
