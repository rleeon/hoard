-- Supabase objects the cloud schema refers to but does not own.
--
-- The cloud tables were born inside Supabase: 14 RLS policies call auth.uid(),
-- one names the authenticated role, 0013 hung profiles off auth.users, and the
-- three admin_metrics functions read both. The server itself never needs any
-- of it (it connects as the table owner, and RLS does not filter the owner),
-- but a plain Postgres can neither replay the migrations nor restore a dump of
-- prod without these names existing.
--
-- Idempotent. Runs before the first migration and before a restore. Never
-- against Supabase, where all of this is the real thing.

DO $$
DECLARE
    r text;
BEGIN
    FOREACH r IN ARRAY ARRAY['anon', 'authenticated', 'service_role'] LOOP
        IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = r) THEN
            EXECUTE format('CREATE ROLE %I NOLOGIN', r);
        END IF;
    END LOOP;
END $$;

CREATE SCHEMA IF NOT EXISTS auth;

-- Accounts stay in Supabase Auth. cloud::auth_mirror copies them here every
-- hour, only the columns admin_metrics reads.
CREATE TABLE IF NOT EXISTS auth.users (id uuid PRIMARY KEY);
ALTER TABLE auth.users
    ADD COLUMN IF NOT EXISTS email text,
    ADD COLUMN IF NOT EXISTS email_confirmed_at timestamptz,
    ADD COLUMN IF NOT EXISTS banned_until timestamptz,
    ADD COLUMN IF NOT EXISTS raw_app_meta_data jsonb,
    ADD COLUMN IF NOT EXISTS created_at timestamptz,
    ADD COLUMN IF NOT EXISTS last_sign_in_at timestamptz;

-- Supabase's own body: the caller's id from the JWT claims its REST layer puts
-- in the session. Here only the admin metrics route sets them, inside its own
-- transaction, and that is what keeps the 42501 check in admin_metrics working.
CREATE OR REPLACE FUNCTION auth.uid() RETURNS uuid
    LANGUAGE sql STABLE
    AS $$
    SELECT coalesce(
        nullif(current_setting('request.jwt.claim.sub', true), ''),
        nullif(current_setting('request.jwt.claims', true), '')::jsonb ->> 'sub'
    )::uuid
$$;
