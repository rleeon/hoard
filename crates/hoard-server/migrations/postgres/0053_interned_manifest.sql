-- The manifest stops repeating itself.
--
-- `save_version_files` holds one row per file *per version*, so every backup
-- rewrites the whole file list even when a single save changed. The bytes are
-- deduplicated (the CAS keys blobs by sha and refcounts them); the manifest is
-- not deduplicated at all. Measured on production before writing this:
--
--   save                                filas   versiones  contenidos  repeticion
--   resonance-a-plague-tale-legacy     69.482          81       1.122       61,9x
--   silenthill2                        18.526         253         429       43,2x
--   s-t-a-l-k-e-r-2                    64.056         727       2.235       28,7x
--   project-zomboid (4 versions!)     140.585           4      35.150        4,0x
--   global                            629.429                 193.888        3,25x
--
-- In `resonance`, 98,4% of the rows are literal copies of a row already there.
-- `project-zomboid` is the other shape of the same problem: four versions and
-- 140k rows, because the save holds 35k files. A retention cap would never
-- touch it, which is why the version cap was studied and dropped as an answer.
--
-- Interning splits that into a catalogue of distinct file contents per save and
-- a list of references per version. Rehearsed against a restored copy of
-- production, all 629.429 rows:
--
--   save_version_files today                          190 MB
--   catalogue + references                            122 MB    (-68 MB)
--
-- The saving matters less than the shape: the table stops growing with the
-- *number of versions* and starts growing only with genuinely new content.
--
-- ---- what the rehearsal changed
--
-- The first draft of this put `save_id` on the reference rows and keyed them
-- `(save_id, version_num, entry_id)`. Measured, that came to **202 MB, worse
-- than doing nothing**: a 37-byte text id repeated across 629k rows and again
-- inside their primary key. `save_versions` already has a `bigint id`, so the
-- references point at that instead and the row drops from 195 bytes to 48, the
-- key from 53 to 16.
--
-- A speculative `(save_id, sha256)` index on the catalogue was dropped too: it
-- cost 19 MB and nothing needs it. Asking which shas a version references walks
-- `version_files` by `version_id` and then the catalogue by primary key.
--
-- `sha256` is born as `bytea`, the type the old column had to be migrated into.
-- `save_id` is NOT born as `uuid`, and that is sequencing, not oversight: it has
-- to match `saves.id`, which is still `text`, and converting that drags
-- `save_versions.save_id` with it (foreign key types must agree), which drags
-- `save_version_files.save_id`, which is exactly the 190 MB rewrite this design
-- exists to avoid. The order is the other way round: kill the big table first,
-- then convert the small ones (`saves` is 1.866 rows) in a later migration,
-- where the only obstacle left is dropping and recreating two RLS policies.
-- Waiting costs ~10 MB.
--
-- This migration only creates the tables. They stay empty and unread: the
-- cutover is a sequence of separate deploys (dual write, backfill, verify,
-- switch reads, drop the old table), each reversible on its own, and the space
-- only comes back at the `DROP TABLE` several steps later. That returns it
-- instantly, with no transient copy, which is the whole reason for this shape
-- over an in-place `ALTER`: there is no room to rewrite a 190 MB table.

CREATE TABLE IF NOT EXISTS public.file_entries (
    id            bigserial PRIMARY KEY,
    save_id       text   NOT NULL REFERENCES public.saves(id) ON DELETE CASCADE,
    relative_path text   NOT NULL,
    sha256        bytea  NOT NULL,
    size_bytes    bigint NOT NULL,
    -- The catalogue key, and the only index this table needs. Scoped per save
    -- rather than globally: a path means nothing outside its save, and a global
    -- unique index would be both larger and a contention point on every commit.
    UNIQUE (save_id, relative_path, sha256)
);

CREATE TABLE IF NOT EXISTS public.version_files (
    version_id  bigint NOT NULL REFERENCES public.save_versions(id) ON DELETE CASCADE,
    entry_id    bigint NOT NULL REFERENCES public.file_entries(id) ON DELETE RESTRICT,
    -- Source mtime, preserved on restore. It lives here and not in the
    -- catalogue because it belongs to the version, not to the content: the same
    -- bytes at the same path can arrive with different timestamps.
    modified_at bigint,
    PRIMARY KEY (version_id, entry_id)
);

-- `ON DELETE RESTRICT` on `entry_id` is deliberate: a catalogue row must not
-- disappear while a version still points at it. Entries are collected once the
-- last version referencing them goes, which is a decision for a sweep, not for
-- a cascade that would silently empty somebody's manifest.
