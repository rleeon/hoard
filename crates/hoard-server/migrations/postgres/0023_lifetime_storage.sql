-- Lifetime ("forever") storage counter.
--
-- `profiles.storage_bytes` is the CURRENT deduped footprint: it grows on
-- upload and is credited back when a version/blob is deleted or purged. So it
-- can only ever tell you what you're holding right now, never what you've
-- hoarded over time.
--
-- `lifetime_storage_bytes` is monotonic: it counts each byte the moment it
-- first enters the hoard (a new archive version, or a blob's first reference)
-- and NEVER subtracts on delete/purge. That lets the recap's "Atesorado" show
-- everything you've ever stored — including saves you later deleted.
--
-- Backfill is the current footprint: already-deleted bytes left no record
-- (delete_version drops the row, the retention cron purges trash), so they
-- can't be recovered. The counter is only exact from this migration forward.

ALTER TABLE profiles
    ADD COLUMN IF NOT EXISTS lifetime_storage_bytes BIGINT NOT NULL DEFAULT 0;

UPDATE profiles
    SET lifetime_storage_bytes = GREATEST(storage_bytes, 0)
    WHERE lifetime_storage_bytes = 0;

-- Re-define both storage triggers (bodies copied from 0019) to also feed the
-- monotonic counter on every ADD path, and never on a subtract path.

-- Legacy archive versions: charged per save_versions row. Content-addressed
-- rows are skipped here (the blob trigger owns their accounting).
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
                SET storage_bytes = storage_bytes + NEW.size_bytes,
                    lifetime_storage_bytes = lifetime_storage_bytes + NEW.size_bytes
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
        -- Soft-delete and restore move only the CURRENT footprint; the bytes
        -- were already counted toward lifetime at their original INSERT, so
        -- lifetime is untouched here.
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

-- Content-addressed blobs: charged once when a blob gains its first reference
-- (INSERT with refcount > 0, or a 0 -> 1 transition). Lifetime grows on those
-- same first-charge paths and never on the credit-back paths.
CREATE OR REPLACE FUNCTION sync_blob_storage() RETURNS trigger AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        IF NEW.refcount > 0 THEN
            UPDATE profiles
                SET storage_bytes = storage_bytes + NEW.size_bytes,
                    lifetime_storage_bytes = lifetime_storage_bytes + NEW.size_bytes
                WHERE user_id = NEW.user_id;
        END IF;
    ELSIF TG_OP = 'UPDATE' THEN
        IF OLD.refcount = 0 AND NEW.refcount > 0 THEN
            UPDATE profiles
                SET storage_bytes = storage_bytes + NEW.size_bytes,
                    lifetime_storage_bytes = lifetime_storage_bytes + NEW.size_bytes
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
