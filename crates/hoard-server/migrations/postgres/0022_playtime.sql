-- Cloud-persisted playtime: real hours played, attributed per local day and
-- per game, per device. The desktop agent already tracks this locally
-- (`hoard-agent::playtime`); this table mirrors it to the cloud so the recap
-- (hoard-wrapple) can read a single, device-merged history — "played X days,
-- Y hours each day, this day Z hours to game W" — across every machine the
-- user signs in from.
--
-- Grain: one row per (user, device, day, game). The agent uploads its full
-- breakdown and the server upserts; a higher count always wins (the client is
-- the source of truth for its own device, and counts only grow). Days that
-- predate the per-game breakdown arrive under the synthetic slug `__other__`
-- so historical day totals stay correct without inventing a game.
--
-- The recap GETs an aggregate: SUM(secs) grouped by day (across devices and
-- games) and by game (across devices and days). device_fp keeps two machines
-- that played the *same* game on the *same* day from clobbering each other.

CREATE TABLE IF NOT EXISTS playtime (
    user_id     UUID NOT NULL REFERENCES profiles(user_id) ON DELETE CASCADE,
    device_fp   TEXT NOT NULL,            -- device fingerprint (logship identity)
    day         DATE NOT NULL,            -- local day the time was attributed to
    game_slug   TEXT NOT NULL,            -- tracked-save slug, or '__other__'
    secs        BIGINT NOT NULL CHECK (secs >= 0),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, device_fp, day, game_slug)
);

CREATE INDEX IF NOT EXISTS idx_playtime_user_day ON playtime(user_id, day);

-- Defense in depth, mirroring 0010_rls.sql / 0021_pro_trials.sql: the server
-- talks to Postgres with the service-role connection (bypasses RLS) and scopes
-- every query by user_id in code. This policy only bites if the Supabase anon
-- key ever reaches the table directly.
ALTER TABLE playtime ENABLE ROW LEVEL SECURITY;
CREATE POLICY playtime_self ON playtime
    FOR SELECT USING (auth.uid() = user_id);
