-- The interned manifest, wearing the shape of the old table.
--
-- Sixteen queries read `save_version_files`, and rewriting each of them into a
-- three-way join would be sixteen chances to get a join condition subtly wrong
-- in a module whose SQL the compiler never checks. A view moves that risk into
-- one place: the call sites change a table name and nothing else, and the join
-- is written once where it can be read and reviewed on its own.
--
-- Named for what it is rather than after the old table, because both exist
-- during the cutover and a name that resolves to two different things depending
-- on the deploy is how you lose an afternoon.
CREATE OR REPLACE VIEW public.manifest_files AS
SELECT v.save_id,
       v.version_num,
       e.relative_path,
       e.sha256,
       e.size_bytes,
       vf.modified_at
  FROM public.version_files vf
  JOIN public.save_versions v ON v.id = vf.version_id
  JOIN public.file_entries e  ON e.id = vf.entry_id;
