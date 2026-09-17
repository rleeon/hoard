-- One fingerprint per kind of object in the cloud schemas, to prove two
-- databases carry the same schema without shipping the schema itself. Run it
-- on both sides and diff. Where a kind differs, swap the last SELECT for
--
--   SELECT kind, name, left(md5(def), 12) FROM objs ORDER BY 1, 2;
--
-- to see which object. Names are resolved through the search_path, so run
-- both sides with `public` on it.

WITH s AS (
    SELECT oid, nspname FROM pg_namespace WHERE nspname IN ('public', 'ops')
),
objs AS (
    SELECT 'table' AS kind,
           s.nspname || '.' || c.relname AS name,
           concat_ws(':', c.relkind, c.relrowsecurity, c.relforcerowsecurity, c.relreplident,
               (SELECT string_agg(concat_ws(' ', a.attname, format_type(a.atttypid, a.atttypmod),
                                            a.attnotnull, pg_get_expr(d.adbin, d.adrelid), co.collname),
                                  ', ' ORDER BY a.attname)
                  FROM pg_attribute a
                  LEFT JOIN pg_attrdef d ON d.adrelid = a.attrelid AND d.adnum = a.attnum
                  LEFT JOIN pg_collation co ON co.oid = a.attcollation
                 WHERE a.attrelid = c.oid AND a.attnum > 0 AND NOT a.attisdropped)) AS def
      FROM pg_class c JOIN s ON s.oid = c.relnamespace
     WHERE c.relkind IN ('r', 'p', 'v', 'm')
    UNION ALL
    SELECT 'index', s.nspname || '.' || ic.relname, pg_get_indexdef(i.indexrelid)
      FROM pg_index i
      JOIN pg_class ic ON ic.oid = i.indexrelid
      JOIN s ON s.oid = ic.relnamespace
    UNION ALL
    SELECT 'constraint', s.nspname || '.' || coalesce(tc.relname, '-') || '.' || con.conname,
           pg_get_constraintdef(con.oid)
      FROM pg_constraint con
      JOIN s ON s.oid = con.connamespace
      LEFT JOIN pg_class tc ON tc.oid = con.conrelid
    UNION ALL
    SELECT 'policy', p.schemaname || '.' || p.tablename || '.' || p.policyname,
           concat_ws(' ', p.permissive, array_to_string(p.roles, ','), p.cmd, p.qual, p.with_check)
      FROM pg_policies p
     WHERE p.schemaname IN ('public', 'ops')
    UNION ALL
    SELECT 'function', s.nspname || '.' || p.proname || '(' || pg_get_function_identity_arguments(p.oid) || ')',
           pg_get_functiondef(p.oid)
      FROM pg_proc p JOIN s ON s.oid = p.pronamespace
     WHERE NOT EXISTS (SELECT 1 FROM pg_depend d WHERE d.objid = p.oid AND d.deptype = 'e')
    UNION ALL
    SELECT 'trigger', s.nspname || '.' || c.relname || '.' || t.tgname, pg_get_triggerdef(t.oid)
      FROM pg_trigger t
      JOIN pg_class c ON c.oid = t.tgrelid
      JOIN s ON s.oid = c.relnamespace
     WHERE NOT t.tgisinternal
    UNION ALL
    SELECT 'view', v.schemaname || '.' || v.viewname, v.definition
      FROM pg_views v
     WHERE v.schemaname IN ('public', 'ops')
    UNION ALL
    SELECT 'sequence', q.schemaname || '.' || q.sequencename, q.data_type::text || ' ' || q.increment_by
      FROM pg_sequences q
     WHERE q.schemaname IN ('public', 'ops')
)
SELECT kind, count(*) AS n, left(md5(string_agg(name || '=' || md5(def), ',' ORDER BY name)), 12) AS fp
  FROM objs
 GROUP BY kind
 ORDER BY kind;
