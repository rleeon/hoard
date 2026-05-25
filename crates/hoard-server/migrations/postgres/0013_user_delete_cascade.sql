-- Wire `profiles.user_id` to `auth.users.id` with ON DELETE CASCADE.
--
-- 0001_profiles.sql declared this in a comment but never actually added the
-- FK — so deleting a row from `auth.users` (via Supabase Auth admin API or
-- a direct DELETE) left `profiles` (and everything that cascades from it:
-- `saves`, `save_versions`, `subscriptions`, `devices`, `sync_log`,
-- `export_jobs`) orphaned in `public`. Detected during a smoke test of
-- /v1/cloud/saves where the auth row was gone but its data wasn't.
--
-- The existing FKs already cascade correctly through `profiles`:
--   subscriptions, devices, saves, sync_log, export_jobs  ->  CASCADE
--   save_versions                                          ->  CASCADE via saves
--   audit_log                                              ->  SET NULL
-- so adding this single edge from `profiles` to `auth.users` is enough:
-- a hard-delete of the auth row now propagates through the whole cloud
-- schema, and `audit_log` keeps its historical rows with a nulled
-- `user_id` for trazabilidad.

ALTER TABLE public.profiles
    ADD CONSTRAINT profiles_user_id_fkey
    FOREIGN KEY (user_id)
    REFERENCES auth.users(id)
    ON DELETE CASCADE;
