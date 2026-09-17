-- What a Supabase database has on top of supabase-stub.sql that changes what
-- pg_dump emits: extensions in their own schema, the postgres role searching
-- it, the Realtime publication, and the ops table made by hand on 2026-08-13
-- (read by admin_metrics_extra). Only for the local cutover rehearsal, where
-- a plain Postgres plays the part of prod.

CREATE SCHEMA IF NOT EXISTS extensions;
CREATE EXTENSION IF NOT EXISTS pgcrypto SCHEMA extensions;
CREATE EXTENSION IF NOT EXISTS "uuid-ossp" SCHEMA extensions;
CREATE EXTENSION IF NOT EXISTS pg_stat_statements SCHEMA extensions;
ALTER ROLE postgres SET search_path = "$user", public, extensions;

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_publication WHERE pubname = 'supabase_realtime') THEN
        CREATE PUBLICATION supabase_realtime;
    END IF;
END $$;

CREATE SCHEMA IF NOT EXISTS ops;
CREATE TABLE IF NOT EXISTS ops.grant_1gb_20260813 (
    user_id uuid,
    plan text,
    storage_limit_bytes bigint,
    saved_at timestamptz
);
