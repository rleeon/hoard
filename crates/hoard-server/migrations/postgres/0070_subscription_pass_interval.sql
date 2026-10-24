-- A Pro pass is paid once and lasts a year: it lands in `subscriptions` like any
-- plan, with interval 'once' and `renews_at` set to the day it runs out, so the
-- expiry `/v1/me` already does for lapsed subscriptions covers it too.
--
-- The CHECK from 0002 was declared inline, so Postgres named it. Dropped by what
-- it checks rather than by that name: if the name ever differed, a DROP ... IF
-- EXISTS would do nothing, the new constraint would go in beside the old one, and
-- the first pass sold would still be rejected.
DO $$
DECLARE
    c text;
BEGIN
    FOR c IN
        SELECT conname FROM pg_constraint
         WHERE conrelid = 'public.subscriptions'::regclass
           AND contype = 'c'
           AND pg_get_constraintdef(oid) LIKE '%interval%'
    LOOP
        EXECUTE format('ALTER TABLE public.subscriptions DROP CONSTRAINT %I', c);
    END LOOP;
END $$;

ALTER TABLE public.subscriptions
    ADD CONSTRAINT subscriptions_interval_check
    CHECK (interval IN ('month', 'year', 'once'));
