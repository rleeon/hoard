-- Content-addressed (per-file SHA-256) cloud storage.
--
-- Until now every cloud version was one opaque `.tar.zst` blob in R2: the
-- client re-packed and re-uploaded the *entire* save folder on every backup,
-- even when a single file changed. For a 600 MB Victoria 3 / Factorio save
-- rewritten on a timer that meant re-uploading 600 MB per cycle and draining
-- the 15-minute bandwidth window.
--
-- The new model stores each file once, keyed by its whole-file SHA-256
-- (`blobs/{user}/{sha[0:2]}/{sha}` in R2), with a per-version manifest that
-- maps relative paths to blobs. The client only uploads files the server
-- doesn't already have. Files are *not* decompressed — the `.v3` zips the
-- game writes are deduped whole.
--
-- Backwards compatible: existing opaque-archive versions keep
-- `content_addressed = false` and their old upload/download/storage paths
-- untouched. Only new uploads use the content-addressed flow.

-- Marks a version as content-addressed (manifest + blobs) vs. legacy archive.
ALTER TABLE save_versions
    ADD COLUMN IF NOT EXISTS content_addressed BOOLEAN NOT NULL DEFAULT FALSE;

-- One row per (user, sha256). Deduped across every save and version the user
-- owns. `refcount` counts how many distinct versions reference the blob; the
-- R2 object + this row are deleted when it hits 0.
CREATE TABLE IF NOT EXISTS cloud_blobs (
    user_id     UUID   NOT NULL REFERENCES profiles(user_id) ON DELETE CASCADE,
    sha256      TEXT   NOT NULL,
    size_bytes  BIGINT NOT NULL,
    r2_key      TEXT   NOT NULL,
    refcount    BIGINT NOT NULL DEFAULT 0,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, sha256)
);

-- Per-version file manifest: which files (by relative path) a content-
-- addressed version is made of, and which blob each maps to.
CREATE TABLE IF NOT EXISTS save_version_files (
    save_id        TEXT   NOT NULL,
    version_num    BIGINT NOT NULL,
    relative_path  TEXT   NOT NULL,
    sha256         TEXT   NOT NULL,
    size_bytes     BIGINT NOT NULL,
    -- Source file mtime (unix seconds). Preserved on restore so a cloud pull
    -- doesn't always look strictly newer than the local copy and silently win
    -- the conflict-aware auto-restore diff. NULL if the FS didn't report one.
    modified_at    BIGINT,
    PRIMARY KEY (save_id, version_num, relative_path),
    FOREIGN KEY (save_id, version_num)
        REFERENCES save_versions(save_id, version_num) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_svf_version ON save_version_files(save_id, version_num);

-- ---------------------------------------------------------------------------
-- Storage accounting reconciliation.
--
-- `profiles.storage_bytes` is the user's real, deduped footprint. For legacy
-- archive versions that's the sum of `save_versions.size_bytes` (each blob is
-- unique). For content-addressed versions `save_versions.size_bytes` is the
-- *logical* size (sum of file sizes) which double-counts bytes shared across
-- versions — so those rows must NOT feed the counter. Instead each unique blob
-- charges storage once, on its first reference, via the cloud_blobs trigger.
-- ---------------------------------------------------------------------------

-- Guarded re-definition: content-addressed save_versions rows are skipped.
CREATE OR REPLACE FUNCTION sync_storage_bytes() RETURNS trigger AS $$
DECLARE
    target_user UUID;
    delta BIGINT;
BEGIN
    IF TG_OP = 'INSERT' THEN
        IF NEW.content_addressed THEN RETURN NULL; END IF;
        SELECT user_id INTO target_user FROM saves WHERE id = NEW.save_id;
        IF target_user IS NOT NULL THEN
            UPDATE profiles
                SET storage_bytes = storage_bytes + NEW.size_bytes
                WHERE user_id = target_user;
        END IF;
    ELSIF TG_OP = 'DELETE' THEN
        IF OLD.content_addressed THEN RETURN NULL; END IF;
        SELECT user_id INTO target_user FROM saves WHERE id = OLD.save_id;
        IF target_user IS NOT NULL THEN
            UPDATE profiles
                SET storage_bytes = GREATEST(0, storage_bytes - OLD.size_bytes)
                WHERE user_id = target_user;
        END IF;
    ELSIF TG_OP = 'UPDATE' THEN
        IF NEW.content_addressed THEN RETURN NULL; END IF;
        IF OLD.deleted_at IS NULL AND NEW.deleted_at IS NOT NULL THEN
            delta := -OLD.size_bytes;
        ELSIF OLD.deleted_at IS NOT NULL AND NEW.deleted_at IS NULL THEN
            delta := NEW.size_bytes;
        ELSE
            delta := NEW.size_bytes - OLD.size_bytes;
        END IF;
        IF delta <> 0 THEN
            SELECT user_id INTO target_user FROM saves WHERE id = NEW.save_id;
            IF target_user IS NOT NULL THEN
                UPDATE profiles
                    SET storage_bytes = GREATEST(0, storage_bytes + delta)
                    WHERE user_id = target_user;
            END IF;
        END IF;
    END IF;
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;
ALTER FUNCTION public.sync_storage_bytes() SET search_path = public, pg_temp;

-- Charge storage when a blob gains its first reference, credit when it loses
-- its last. The commit path UPSERTs (refcount +1), delete paths decrement.
CREATE OR REPLACE FUNCTION sync_blob_storage() RETURNS trigger AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        IF NEW.refcount > 0 THEN
            UPDATE profiles SET storage_bytes = storage_bytes + NEW.size_bytes
                WHERE user_id = NEW.user_id;
        END IF;
    ELSIF TG_OP = 'UPDATE' THEN
        IF OLD.refcount = 0 AND NEW.refcount > 0 THEN
            UPDATE profiles SET storage_bytes = storage_bytes + NEW.size_bytes
                WHERE user_id = NEW.user_id;
        ELSIF OLD.refcount > 0 AND NEW.refcount = 0 THEN
            UPDATE profiles SET storage_bytes = GREATEST(0, storage_bytes - NEW.size_bytes)
                WHERE user_id = NEW.user_id;
        END IF;
    ELSIF TG_OP = 'DELETE' THEN
        IF OLD.refcount > 0 THEN
            UPDATE profiles SET storage_bytes = GREATEST(0, storage_bytes - OLD.size_bytes)
                WHERE user_id = OLD.user_id;
        END IF;
    END IF;
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;
ALTER FUNCTION public.sync_blob_storage() SET search_path = public, pg_temp;

CREATE TRIGGER trg_blobs_storage_ins
    AFTER INSERT ON cloud_blobs
    FOR EACH ROW EXECUTE FUNCTION sync_blob_storage();
CREATE TRIGGER trg_blobs_storage_upd
    AFTER UPDATE ON cloud_blobs
    FOR EACH ROW EXECUTE FUNCTION sync_blob_storage();
CREATE TRIGGER trg_blobs_storage_del
    AFTER DELETE ON cloud_blobs
    FOR EACH ROW EXECUTE FUNCTION sync_blob_storage();

-- RLS parity with the other tables (service role bypasses; defense in depth).
ALTER TABLE cloud_blobs        ENABLE ROW LEVEL SECURITY;
ALTER TABLE save_version_files ENABLE ROW LEVEL SECURITY;

CREATE POLICY cloud_blobs_self ON cloud_blobs
    FOR ALL USING (auth.uid() = user_id);

CREATE POLICY save_version_files_self ON save_version_files
    FOR ALL USING (
        save_id IN (SELECT id FROM saves WHERE user_id = auth.uid())
    );
