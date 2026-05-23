-- Row-level security. The service-role key (used by the server with the
-- Postgres connection string from Supabase) bypasses RLS, so these policies
-- only matter if/when we use the Supabase anon key from a client directly.
-- They're defense in depth.

ALTER TABLE profiles       ENABLE ROW LEVEL SECURITY;
ALTER TABLE subscriptions  ENABLE ROW LEVEL SECURITY;
ALTER TABLE devices        ENABLE ROW LEVEL SECURITY;
ALTER TABLE saves          ENABLE ROW LEVEL SECURITY;
ALTER TABLE save_versions  ENABLE ROW LEVEL SECURITY;
ALTER TABLE sync_log       ENABLE ROW LEVEL SECURITY;
ALTER TABLE export_jobs    ENABLE ROW LEVEL SECURITY;

-- profiles: a user sees only their own row.
CREATE POLICY profiles_self_read ON profiles
    FOR SELECT USING (auth.uid() = user_id);
CREATE POLICY profiles_self_update ON profiles
    FOR UPDATE USING (auth.uid() = user_id);

-- subscriptions / devices / saves / save_versions / sync_log / export_jobs:
-- same self-ownership pattern.
CREATE POLICY subs_self ON subscriptions
    FOR SELECT USING (auth.uid() = user_id);

CREATE POLICY devices_self ON devices
    FOR ALL USING (auth.uid() = user_id);

CREATE POLICY saves_self ON saves
    FOR ALL USING (auth.uid() = user_id);

CREATE POLICY versions_self ON save_versions
    FOR ALL USING (
        save_id IN (SELECT id FROM saves WHERE user_id = auth.uid())
    );

CREATE POLICY sync_log_self ON sync_log
    FOR SELECT USING (auth.uid() = user_id);

CREATE POLICY exports_self ON export_jobs
    FOR ALL USING (auth.uid() = user_id);

-- audit_log has no policy: only the server (service role) ever reads/writes.
ALTER TABLE audit_log ENABLE ROW LEVEL SECURITY;
