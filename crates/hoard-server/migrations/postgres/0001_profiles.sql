-- Profiles extend Supabase's auth.users with Hoard-specific fields.
--
-- We don't own auth.users (Supabase Auth does); we reference it by uuid.
-- ON DELETE CASCADE so when a user nukes their account in Supabase, all of
-- their Hoard rows go with it.
--
-- RLS is on by default: a row is only visible to its own user via the
-- Supabase JWT. The server uses the service-role key for admin tasks
-- (webhooks, cron) which bypasses RLS.

CREATE TABLE IF NOT EXISTS profiles (
    user_id        UUID PRIMARY KEY,
    email          TEXT NOT NULL,
    display_name   TEXT,
    avatar_url     TEXT,
    plan           TEXT NOT NULL DEFAULT 'free'
                       CHECK (plan IN ('free', 'pro', 'proplus')),
    storage_bytes  BIGINT NOT NULL DEFAULT 0,
    devices_count  INTEGER NOT NULL DEFAULT 0,
    deleted_at     TIMESTAMPTZ,         -- soft-delete, 30d grace
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_profiles_plan        ON profiles(plan);
CREATE INDEX IF NOT EXISTS idx_profiles_deleted_at  ON profiles(deleted_at)
    WHERE deleted_at IS NOT NULL;

-- updated_at trigger (shared helper, see 0009_triggers.sql)
