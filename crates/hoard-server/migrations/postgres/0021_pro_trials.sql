-- Per-feature Pro trials for the open-core features (hoard-screen,
-- hoard-wrapple). The build-time `pro` feature only decides whether the Pro
-- code is compiled into the official binary; the *real* lock is server-side:
-- every Pro content endpoint calls `entitlements::require_feature`, which on a
-- Free account starts (on first use) and then enforces a one-month trial,
-- returning 402 once it elapses. Paid Pro skips the trial entirely.
--
-- The trial clock lives here, not on the client, so tampering with the device
-- clock or patching the OSS binary can't extend it. One row per (user,
-- feature): the first content request inserts it (ON CONFLICT DO NOTHING so the
-- window never restarts), later requests compare now() to expires_at.

CREATE TABLE IF NOT EXISTS pro_trials (
    user_id     UUID NOT NULL REFERENCES profiles(user_id) ON DELETE CASCADE,
    feature     TEXT NOT NULL CHECK (feature IN ('screen', 'wrapple')),
    started_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at  TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (user_id, feature)
);

CREATE INDEX IF NOT EXISTS idx_pro_trials_expires ON pro_trials(expires_at);

-- Defense in depth, mirroring 0010_rls.sql: the server uses the service-role
-- Postgres connection (bypasses RLS); this policy only bites if the Supabase
-- anon key ever touches the table directly.
ALTER TABLE pro_trials ENABLE ROW LEVEL SECURITY;
CREATE POLICY pro_trials_self ON pro_trials
    FOR SELECT USING (auth.uid() = user_id);
