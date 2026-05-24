-- Mitigate the Supabase security linter:
--   1. `function_search_path_mutable` — pin search_path on the helper
--      functions so they always resolve names from `public` (defence
--      in depth against schema-shadow attacks via search_path).
--   2. `rls_enabled_no_policy` on audit_log — RLS is on but no policy
--      existed, so anon/auth roles would have been denied implicitly.
--      Make the deny explicit so the linter stays green; the server
--      uses the service-role (bypasses RLS) for audit writes.

ALTER FUNCTION public.set_updated_at()       SET search_path = public, pg_temp;
ALTER FUNCTION public.sync_storage_bytes()   SET search_path = public, pg_temp;

CREATE POLICY audit_log_deny_all ON public.audit_log
    FOR ALL USING (false);
