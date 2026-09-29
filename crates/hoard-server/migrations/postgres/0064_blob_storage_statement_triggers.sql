-- Blob storage accounting, once per statement instead of once per row.
--
-- The row triggers from 0019 (lifetime counter from 0023) update `profiles`
-- once for every blob that reaches or leaves refcount 0. Inside one
-- transaction every one of those updates writes a new version of the same
-- profile row, the old versions can't be pruned until commit, and each update
-- walks the whole chain to find the live one: quadratic. Handing back the
-- 20,000 references of one large save took 38 s on a laptop; the largest save
-- in production names 24,791 blobs. Committing a version that big pays the
-- same on the way in.
--
-- The same arithmetic, summed per account from the statement's transition
-- tables: one `profiles` update per account per statement, whatever the row
-- count. `sync_blob_storage()` stays defined, unused, so going back is a
-- migration that only recreates the old triggers.

CREATE OR REPLACE FUNCTION sync_blob_storage_inserted() RETURNS trigger AS $$
BEGIN
    UPDATE profiles p
       SET storage_bytes = p.storage_bytes + d.up,
           lifetime_storage_bytes = p.lifetime_storage_bytes + d.up
      FROM (SELECT user_id, sum(size_bytes) AS up
              FROM new_rows WHERE refcount > 0
             GROUP BY user_id) d
     WHERE p.user_id = d.user_id;
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;
ALTER FUNCTION public.sync_blob_storage_inserted() SET search_path = public, pg_temp;

-- A row can only cross zero one way per statement, so an account's `up` and
-- `down` come from different rows. The floor at 0 is applied once to the net
-- change rather than per row; the two only differ for an account whose
-- counter had already drifted below its blobs.
CREATE OR REPLACE FUNCTION sync_blob_storage_updated() RETURNS trigger AS $$
BEGIN
    UPDATE profiles p
       SET storage_bytes = GREATEST(0, p.storage_bytes + d.up - d.down),
           lifetime_storage_bytes = p.lifetime_storage_bytes + d.up
      FROM (SELECT n.user_id,
                   coalesce(sum(n.size_bytes) FILTER (WHERE o.refcount = 0 AND n.refcount > 0), 0) AS up,
                   coalesce(sum(n.size_bytes) FILTER (WHERE o.refcount > 0 AND n.refcount = 0), 0) AS down
              FROM new_rows n
              JOIN old_rows o ON o.user_id = n.user_id AND o.sha256 = n.sha256
             GROUP BY n.user_id) d
     WHERE p.user_id = d.user_id
       AND (d.up <> 0 OR d.down <> 0);
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;
ALTER FUNCTION public.sync_blob_storage_updated() SET search_path = public, pg_temp;

CREATE OR REPLACE FUNCTION sync_blob_storage_deleted() RETURNS trigger AS $$
BEGIN
    UPDATE profiles p
       SET storage_bytes = GREATEST(0, p.storage_bytes - d.down)
      FROM (SELECT user_id, sum(size_bytes) AS down
              FROM old_rows WHERE refcount > 0
             GROUP BY user_id) d
     WHERE p.user_id = d.user_id;
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;
ALTER FUNCTION public.sync_blob_storage_deleted() SET search_path = public, pg_temp;

-- Dropped and created in this one migration transaction: no statement ever
-- runs between the old triggers and the new ones.
DROP TRIGGER IF EXISTS trg_blobs_storage_ins ON cloud_blobs;
DROP TRIGGER IF EXISTS trg_blobs_storage_upd ON cloud_blobs;
DROP TRIGGER IF EXISTS trg_blobs_storage_del ON cloud_blobs;

CREATE TRIGGER trg_blobs_storage_ins
    AFTER INSERT ON cloud_blobs
    REFERENCING NEW TABLE AS new_rows
    FOR EACH STATEMENT EXECUTE FUNCTION sync_blob_storage_inserted();
CREATE TRIGGER trg_blobs_storage_upd
    AFTER UPDATE ON cloud_blobs
    REFERENCING OLD TABLE AS old_rows NEW TABLE AS new_rows
    FOR EACH STATEMENT EXECUTE FUNCTION sync_blob_storage_updated();
CREATE TRIGGER trg_blobs_storage_del
    AFTER DELETE ON cloud_blobs
    REFERENCING OLD TABLE AS old_rows
    FOR EACH STATEMENT EXECUTE FUNCTION sync_blob_storage_deleted();
