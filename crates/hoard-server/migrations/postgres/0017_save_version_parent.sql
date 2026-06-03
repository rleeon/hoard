-- DAG provenance for cloud save_versions (mirror of SQLite 0015).
-- NULL = root version. Lets init_upload reject a divergent push with
-- 409 non_fast_forward when another device advanced latest_version_num
-- since the client's declared base version.
ALTER TABLE save_versions ADD COLUMN parent_version BIGINT;
