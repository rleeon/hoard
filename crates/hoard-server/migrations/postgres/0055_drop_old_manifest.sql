-- Phase 3, step 6: the old per-version manifest goes away.
--
-- `save_version_files` held one row per file **per version**: 635.598 rows for
-- 194.195 distinct contents, a 3,3x repetition globally and 61,9x on the worst
-- save. Migration 0053 replaced it with `file_entries` (the catalogue, each
-- distinct (path, sha) once) plus `version_files` (one narrow reference per
-- file per version), 0054 pointed every read at the `manifest_files` view, and
-- the backfill filled in everything older than the dual write.
--
-- Both shapes were compared in production, in both directions, over all
-- 635.598 rows: zero rows the interned pair cannot reproduce, zero it invented.
-- The server no longer writes the old table either.
--
-- This is where the space comes back, and it comes back at once: `DROP TABLE`
-- returns 197 MB immediately, with no `VACUUM FULL` and no transient copy.
-- That property is the reason phase 3 built new tables instead of altering the
-- old one in place: the `ALTER` did not fit under the 500 MB ceiling.

-- ---------------------------------------------------------------- admin_metrics
--
-- `admin_metrics()` reads the old table in two places. plpgsql does not resolve
-- table names until the function runs, so the `DROP` below would succeed and
-- the admin dashboard would start failing at the next click instead, the kind
-- of breakage that surfaces days later with no obvious cause.
--
-- The body is rewritten from whatever is *live*, not from a copy of 0039: the
-- deployed function has picked up at least one change applied by hand
-- (`statement_timeout = 30s`), and a `create or replace` written out longhand
-- would silently revert it. `pg_get_functiondef` returns the real thing, the
-- two subqueries are swapped by text, and the result is checked before it is
-- executed. Verified equal on production data first: both shapes report
-- 635.598 rows and 511.696.354.489 logical bytes.
do $$
declare
  def     text;
  new_def text;
begin
  if to_regprocedure('public.admin_metrics()') is null then
    return;
  end if;

  def := pg_get_functiondef('public.admin_metrics()'::regprocedure);

  new_def := replace(
    def,
    '(select coalesce(sum(size_bytes), 0) from save_version_files)',
    '(select coalesce(sum(e.size_bytes), 0) from version_files vf'
      || ' join file_entries e on e.id = vf.entry_id)');
  new_def := replace(
    new_def,
    '(select count(*) from save_version_files)',
    '(select count(*) from version_files)');

  -- Loud, not silent: if the body ever stops matching these two strings the
  -- migration stops the deploy rather than dropping the table out from under a
  -- function that still reads it.
  if new_def like '%save_version_files%' then
    raise exception
      'admin_metrics() still references save_version_files after the rewrite';
  end if;

  execute new_def;
end
$$;

-- --------------------------------------------------------------------- RLS
--
-- Production has row level security switched on for both interned tables, but
-- 0053 never said so: it was enabled by hand, so a database rebuilt from these
-- migrations would come up with the manifest readable through PostgREST by any
-- authenticated user. Said here, once, so the schema carries it. Idempotent,
-- and it is what production already has.
ALTER TABLE public.file_entries  ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.version_files ENABLE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------- the drop
--
-- Its `save_version_files_self` policy and its two indexes (the 118 MB primary
-- key among them) go with it.
DROP TABLE public.save_version_files;
