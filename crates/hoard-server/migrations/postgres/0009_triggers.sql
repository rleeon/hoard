-- Helper: bump updated_at on UPDATE.
CREATE OR REPLACE FUNCTION set_updated_at() RETURNS trigger AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_profiles_updated_at
    BEFORE UPDATE ON profiles
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE TRIGGER trg_subscriptions_updated_at
    BEFORE UPDATE ON subscriptions
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE TRIGGER trg_saves_updated_at
    BEFORE UPDATE ON saves
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

-- storage_bytes coherence: maintain profiles.storage_bytes as the sum of
-- non-deleted save_versions.size_bytes for the user. Faster than recomputing
-- on every quota check.
CREATE OR REPLACE FUNCTION sync_storage_bytes() RETURNS trigger AS $$
DECLARE
    target_user UUID;
    delta BIGINT;
BEGIN
    IF TG_OP = 'INSERT' THEN
        SELECT user_id INTO target_user FROM saves WHERE id = NEW.save_id;
        IF target_user IS NOT NULL THEN
            UPDATE profiles
                SET storage_bytes = storage_bytes + NEW.size_bytes
                WHERE user_id = target_user;
        END IF;
    ELSIF TG_OP = 'DELETE' THEN
        SELECT user_id INTO target_user FROM saves WHERE id = OLD.save_id;
        IF target_user IS NOT NULL THEN
            UPDATE profiles
                SET storage_bytes = GREATEST(0, storage_bytes - OLD.size_bytes)
                WHERE user_id = target_user;
        END IF;
    ELSIF TG_OP = 'UPDATE' THEN
        -- Soft-delete flips deleted_at; treat as removal/restore.
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

CREATE TRIGGER trg_versions_storage_ins
    AFTER INSERT ON save_versions
    FOR EACH ROW EXECUTE FUNCTION sync_storage_bytes();

CREATE TRIGGER trg_versions_storage_upd
    AFTER UPDATE ON save_versions
    FOR EACH ROW EXECUTE FUNCTION sync_storage_bytes();

CREATE TRIGGER trg_versions_storage_del
    AFTER DELETE ON save_versions
    FOR EACH ROW EXECUTE FUNCTION sync_storage_bytes();
