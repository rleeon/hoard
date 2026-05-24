-- The sqlx migrator creates `_sqlx_migrations` as its first step and
-- (since Supabase auto-enables RLS on `public`) leaves it with no
-- policies. The Supabase linter then flags `rls_enabled_no_policy`.
--
-- The server connects as the table owner (`postgres`) which bypasses
-- RLS anyway, so this is purely about keeping the linter green: a
-- single deny-all policy makes the table explicitly unreachable for
-- any other role.

CREATE POLICY sqlx_migrations_deny_all ON public._sqlx_migrations
    FOR ALL USING (false);
