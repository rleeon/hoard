-- The per-save cap becomes the user's to move.
--
-- Free is sized around 1 GB per save and that stays the default. What it could
-- not do is fit the one save that matters to somebody: a single monolithic
-- ~1.2 GB file bounces off the cap on every autosave, and the client trims it
-- to the newest files that fit, which for a one-file save means it never syncs
-- whole. The answer that costs nothing is letting them ask for more room per
-- save inside the same 2 GB account: the limit that maps to a bill is the
-- total, not the shape of what fills it.
--
-- NULL, the default and every existing row, means "the plan's number". A value
-- is clamped on read (`plans::resolved_save_size_limit`) rather than trusted:
-- the range check in the endpoint only speaks for the values it saw, and a cap
-- set while Pro must stop being enforced the moment the subscription lapses,
-- which nothing rewrites the column for.
ALTER TABLE profiles
    ADD COLUMN IF NOT EXISTS max_save_size_bytes BIGINT;

-- A negative value would read as "chosen" and clamp up to the floor, which is
-- not what anybody meant by it. Zero stays legal and reads as NULL does.
ALTER TABLE profiles
    DROP CONSTRAINT IF EXISTS profiles_max_save_size_bytes_nonneg;
ALTER TABLE profiles
    ADD CONSTRAINT profiles_max_save_size_bytes_nonneg
    CHECK (max_save_size_bytes IS NULL OR max_save_size_bytes >= 0);
