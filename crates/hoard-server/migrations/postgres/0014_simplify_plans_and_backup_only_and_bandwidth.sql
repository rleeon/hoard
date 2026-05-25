-- Product decisions consolidated for 1.6.1 (see CHANGELOG entry):
--
-- 1. Plans drop "proplus" entirely. Free moves from 500 MB / 1 device /
--    3 saves / 7-day history → 1 GB / 3 devices / unlimited saves /
--    forever history. Pro moves from 50 GB / 5 devices / 90-day history
--    → 50 GB / unlimited devices / forever. Both gain a per-save size cap
--    (200 MB Free, 2 GB Pro) and a 15-minute rolling bandwidth window
--    (500 MB Free, 1 GB Pro) so we can enforce sane usage on the cheap
--    R2 storage tier without metering each request.
--
-- 2. `saves.backup_only` — per-save flag. When true, the manifest pull
--    (`GET /v1/cloud/sync`) hides the save so other devices won't
--    auto-restore it; uploads still happen, restores stay manual via
--    `/v1/cloud/saves/:id/versions/:n/download`.
--
-- 3. `bandwidth_usage` — minute-bucketed counter consulted on every
--    upload/download to enforce the rolling window. A separate row per
--    user per minute keeps writes lock-free under concurrency (each
--    upload touches its own (user, minute) row) and the SUM-over-window
--    query stays under 15 index seeks. A cron deletes rows older than
--    1 hour so the table stays bounded.

-- ---------------------------------------------------------------
-- (1) plans.proplus -> pro
-- ---------------------------------------------------------------

UPDATE public.profiles
    SET plan = 'pro', updated_at = now()
    WHERE plan = 'proplus';

UPDATE public.subscriptions
    SET plan = 'pro', updated_at = now()
    WHERE plan = 'proplus';

ALTER TABLE public.profiles DROP CONSTRAINT IF EXISTS profiles_plan_check;
ALTER TABLE public.profiles
    ADD CONSTRAINT profiles_plan_check
    CHECK (plan IN ('free', 'pro'));

ALTER TABLE public.subscriptions DROP CONSTRAINT IF EXISTS subscriptions_plan_check;
ALTER TABLE public.subscriptions
    ADD CONSTRAINT subscriptions_plan_check
    CHECK (plan IN ('free', 'pro'));

-- ---------------------------------------------------------------
-- (2) saves.backup_only
-- ---------------------------------------------------------------

ALTER TABLE public.saves
    ADD COLUMN IF NOT EXISTS backup_only BOOLEAN NOT NULL DEFAULT false;

CREATE INDEX IF NOT EXISTS idx_saves_user_backup_only
    ON public.saves (user_id)
    WHERE backup_only = false;

-- ---------------------------------------------------------------
-- (3) bandwidth_usage
-- ---------------------------------------------------------------

CREATE TABLE IF NOT EXISTS public.bandwidth_usage (
    user_id      UUID NOT NULL REFERENCES public.profiles(user_id) ON DELETE CASCADE,
    bucket_start TIMESTAMPTZ NOT NULL,
    bytes_used   BIGINT NOT NULL DEFAULT 0,
    PRIMARY KEY (user_id, bucket_start)
);

CREATE INDEX IF NOT EXISTS idx_bandwidth_user_bucket
    ON public.bandwidth_usage (user_id, bucket_start DESC);

ALTER TABLE public.bandwidth_usage ENABLE ROW LEVEL SECURITY;

-- Only the service-role server reads/writes this table; no policy means
-- direct anon-key reads are denied, which is what we want.
