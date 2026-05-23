-- Subscriptions track Lemon Squeezy state per user.
--
-- One row per active or historical subscription. A user may have multiple
-- rows over time (e.g. cancelled Pro then re-upgraded). The "current"
-- subscription is the one with the latest `renews_at` and `status` in
-- ('active','grace').
--
-- We mirror LS state instead of querying their API on every request — the
-- webhook is the source of truth for `status` transitions.

CREATE TABLE IF NOT EXISTS subscriptions (
    id                    BIGSERIAL PRIMARY KEY,
    user_id               UUID NOT NULL REFERENCES profiles(user_id) ON DELETE CASCADE,
    ls_customer_id        TEXT NOT NULL,
    ls_subscription_id    TEXT NOT NULL UNIQUE,
    ls_variant_id         TEXT NOT NULL,
    plan                  TEXT NOT NULL
                              CHECK (plan IN ('free', 'pro', 'proplus')),
    interval              TEXT NOT NULL
                              CHECK (interval IN ('month', 'year')),
    status                TEXT NOT NULL
                              CHECK (status IN ('active', 'grace', 'cancelled', 'expired')),
    renews_at             TIMESTAMPTZ,
    cancel_at             TIMESTAMPTZ,
    cancelled_at          TIMESTAMPTZ,
    expired_at            TIMESTAMPTZ,
    last_event            TEXT,                       -- LS event name
    last_event_at         TIMESTAMPTZ,
    created_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at            TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_subscriptions_user ON subscriptions(user_id);
CREATE INDEX IF NOT EXISTS idx_subscriptions_status_renews
    ON subscriptions(status, renews_at)
    WHERE status IN ('active', 'grace');
