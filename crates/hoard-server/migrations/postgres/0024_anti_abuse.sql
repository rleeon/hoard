-- Anti-abuse: canonical email (Sybil/alias dedup) + a fingerprint index that
-- backs the per-device free-account cap.
--
-- `email_canonical` collapses Gmail aliasing (dots and `+tags`) and lowercases,
-- so `R.Ai+spam@gmail.com` and `rai@gmail.com` resolve to one identity. The app
-- recomputes it on every profile upsert; this backfills existing rows so the
-- uniqueness guard also covers accounts created before this migration.
--
-- Cloud-only (Postgres). Self-hosted runs the SQLite migrations under
-- `migrations/` and never sees this file, so its no-accounts / no-limits model
-- is untouched.

ALTER TABLE profiles ADD COLUMN IF NOT EXISTS email_canonical TEXT;

-- Backfill. Gmail/googlemail: strip the `+tag`, remove dots from the local
-- part, force the gmail.com domain (Gmail ignores all three). Everything else:
-- lowercase + trim only — we deliberately don't touch `+tags` for other
-- providers, where they aren't always aliases and a false merge could lock out
-- a distinct address. Skip malformed (no `@`) rows.
UPDATE profiles
   SET email_canonical = CASE
       WHEN lower(split_part(btrim(email), '@', 2)) IN ('gmail.com', 'googlemail.com')
           THEN replace(split_part(split_part(lower(btrim(email)), '@', 1), '+', 1), '.', '')
                || '@gmail.com'
       ELSE lower(btrim(email))
   END
 WHERE position('@' in email) > 1
   AND email_canonical IS NULL;

-- If two pre-existing live accounts already collide on the canonical value
-- (the same person under two aliases), keep the oldest and clear the canonical
-- on the rest. They're grandfathered in; the guard below only blocks *new*
-- duplicates. Without this, the unique index could fail to build on a dataset
-- that already contains a collision.
WITH ranked AS (
    SELECT user_id,
           row_number() OVER (
               PARTITION BY email_canonical
               ORDER BY created_at, user_id
           ) AS rn
      FROM profiles
     WHERE email_canonical IS NOT NULL
       AND deleted_at IS NULL
)
UPDATE profiles p
   SET email_canonical = NULL
  FROM ranked r
 WHERE p.user_id = r.user_id
   AND r.rn > 1;

-- One live account per canonical email. Soft-deleted rows are excluded so a
-- user who deletes and re-registers within the 30-day grace isn't permanently
-- locked out of their own address.
CREATE UNIQUE INDEX IF NOT EXISTS uq_profiles_email_canonical_live
    ON profiles (email_canonical)
    WHERE email_canonical IS NOT NULL AND deleted_at IS NULL;

-- Backs the per-device free-account count (fingerprint -> distinct user_ids).
CREATE INDEX IF NOT EXISTS idx_devices_fingerprint ON devices (fingerprint);
