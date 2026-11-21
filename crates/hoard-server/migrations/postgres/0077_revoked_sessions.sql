-- Unlinking a device used to delete its row and nothing else. A machine that
-- was still running re-registered on its next GET /v1/me, so a stolen laptop
-- could not be put out of the account from anywhere.
--
-- Each device now remembers the Supabase session it last spoke with (the
-- `session_id` claim, which survives refreshes), and unlinking it files that
-- session here. The auth middleware refuses its tokens and, the next time the
-- machine calls, ends the session at Supabase with that same token, so the
-- refresh token dies too and the client falls into its "sign in again" path.
--
-- No index on `devices.session_id`: the heartbeat rewrites that table every
-- 30 s per device and 0068 dropped its indexes to keep those updates HOT.

ALTER TABLE devices ADD COLUMN IF NOT EXISTS session_id UUID;

CREATE TABLE IF NOT EXISTS revoked_sessions (
    session_id        UUID        PRIMARY KEY,
    user_id           UUID        NOT NULL REFERENCES profiles(user_id) ON DELETE CASCADE,
    revoked_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- Throttles the call to Supabase: a revoked machine still beats every 30 s.
    logout_attempt_at TIMESTAMPTZ,
    signed_out_at     TIMESTAMPTZ
);

-- Same lockdown as every other per-user table (see 0062).
ALTER TABLE revoked_sessions ENABLE ROW LEVEL SECURITY;

DROP POLICY IF EXISTS revoked_sessions_self ON revoked_sessions;
CREATE POLICY revoked_sessions_self ON revoked_sessions
    FOR ALL
    USING ((SELECT auth.uid()) = user_id)
    WITH CHECK ((SELECT auth.uid()) = user_id);
