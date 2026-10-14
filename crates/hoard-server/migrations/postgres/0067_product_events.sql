-- Product events: the milestones and choices that say how Hoard Cloud gets
-- used, in one table under one naming scheme (`hoard_core::product`).
--
-- Before this, the same story was spread over `audit_log.event_type`,
-- `sync_log.kind` and free text in `client_logs`, each pruned on its own clock.
-- The Pro analyses of aug-oct 2026 were all hand-written SQL against prod
-- joining the three, and the one that asked why 30 checkouts in ten days ended
-- with nobody paying (13-oct) needed Polar's API on top, because nothing here
-- recorded what the app showed before the payment form.
--
-- `dedupe_key` is how "once" and "once a day" are kept: NULL for an event where
-- every occurrence counts (NULLs never collide in a unique index), the scope
-- for a once-ever event, `day|scope` for a daily one. Writers insert with
-- ON CONFLICT DO NOTHING and the index does the rest.
--
-- Kept two years (pruned by the server, `cloud::product_events`) and deleted
-- with the account.

CREATE TABLE IF NOT EXISTS product_events (
    id                 BIGSERIAL   PRIMARY KEY,
    user_id            UUID        NOT NULL REFERENCES profiles(user_id) ON DELETE CASCADE,
    name               TEXT        NOT NULL,
    source             TEXT        NOT NULL CHECK (source IN ('server', 'app', 'backfill')),
    dedupe_key         TEXT,
    props              JSONB       NOT NULL DEFAULT '{}'::jsonb,
    device_fingerprint TEXT,
    app_version        TEXT,
    occurred_at        TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX IF NOT EXISTS uq_product_events_dedupe
    ON product_events (user_id, name, dedupe_key);
CREATE INDEX IF NOT EXISTS idx_product_events_name_time
    ON product_events (name, occurred_at);
CREATE INDEX IF NOT EXISTS idx_product_events_time
    ON product_events (occurred_at);

-- Same lockdown as every other per-user table (see 0062).
ALTER TABLE product_events ENABLE ROW LEVEL SECURITY;

DROP POLICY IF EXISTS product_events_self ON product_events;
CREATE POLICY product_events_self ON product_events
    FOR ALL
    USING ((SELECT auth.uid()) = user_id)
    WITH CHECK ((SELECT auth.uid()) = user_id);

-- ---- backfill
--
-- What the existing tables already know, so the funnel does not start empty.
-- Every row says `source = 'backfill'`. Left out on purpose:
--   * `active.day`: the live event means "the service checked in that day";
--     the only history is sync activity, which is a different question.
--   * storage thresholds: there is no record of when anyone crossed one.
--   * the app's own events: nothing recorded them.
-- `devices` loses machines to the 90-day prune and `sync_log`/`audit_log` keep
-- everything, so device counts here are a floor.

INSERT INTO product_events (user_id, name, source, dedupe_key, occurred_at)
SELECT user_id, 'account.created', 'backfill', '', created_at
  FROM profiles
ON CONFLICT DO NOTHING;

INSERT INTO product_events (user_id, name, source, props, device_fingerprint, app_version, occurred_at)
SELECT user_id, 'device.registered', 'backfill',
       jsonb_build_object(
           'os', os,
           'nth', row_number() OVER (PARTITION BY user_id ORDER BY created_at)),
       fingerprint, app_version, created_at
  FROM devices;

INSERT INTO product_events (user_id, name, source, dedupe_key, occurred_at)
SELECT user_id, 'upload.first', 'backfill', '', min(at)
  FROM sync_log
 WHERE kind = 'upload'
 GROUP BY user_id
ON CONFLICT DO NOTHING;

INSERT INTO product_events (user_id, name, source, dedupe_key, occurred_at)
SELECT user_id, 'download.first', 'backfill', '', min(at)
  FROM sync_log
 WHERE kind = 'download'
 GROUP BY user_id
ON CONFLICT DO NOTHING;

-- One row per account and day, as the live writer keeps them. A client stuck
-- against a full account wrote 26.655 `quota_block` rows in a month (sep-2026);
-- what matters is that it happened that day.
INSERT INTO product_events (user_id, name, source, dedupe_key, props, occurred_at)
SELECT user_id,
       CASE kind WHEN 'quota_block' THEN 'quota.blocked' ELSE 'bandwidth.blocked' END,
       'backfill',
       to_char((at AT TIME ZONE 'UTC')::date, 'YYYY-MM-DD'),
       jsonb_build_object('blocks', count(*)),
       min(at)
  FROM sync_log
 WHERE kind IN ('quota_block', 'bandwidth_block')
 GROUP BY user_id, kind, (at AT TIME ZONE 'UTC')::date
ON CONFLICT DO NOTHING;

INSERT INTO product_events (user_id, name, source, dedupe_key, props, occurred_at)
SELECT user_id, 'save.too_large', 'backfill',
       to_char((at AT TIME ZONE 'UTC')::date, 'YYYY-MM-DD') || '|' || coalesce(metadata->>'save_id', ''),
       jsonb_strip_nulls(jsonb_build_object(
           'game_slug', max(metadata->>'game_slug'),
           'bytes', max(bytes))),
       min(at)
  FROM sync_log
 WHERE kind = 'save_too_large'
 GROUP BY user_id, (at AT TIME ZONE 'UTC')::date, metadata->>'save_id'
ON CONFLICT DO NOTHING;

INSERT INTO product_events (user_id, name, source, props, occurred_at)
SELECT user_id, 'checkout.requested', 'backfill',
       jsonb_strip_nulls(coalesce(metadata, '{}'::jsonb)), created_at
  FROM audit_log
 WHERE event_type = 'checkout.requested'
   AND user_id IS NOT NULL;

INSERT INTO product_events (user_id, name, source, dedupe_key, props, occurred_at)
SELECT user_id, 'checkout.status', 'backfill',
       coalesce(metadata->>'checkout_id', '') || ':' || coalesce(metadata->>'status', ''),
       jsonb_strip_nulls(jsonb_build_object(
           'checkout_id', metadata->>'checkout_id',
           'status', metadata->>'status')),
       created_at
  FROM audit_log
 WHERE actor = 'polar'
   AND event_type LIKE 'checkout.%'
   AND user_id IS NOT NULL
ON CONFLICT DO NOTHING;

INSERT INTO product_events (user_id, name, source, dedupe_key, props, occurred_at)
SELECT user_id, 'subscription.started', 'backfill', ls_subscription_id,
       jsonb_build_object('plan', plan, 'interval', interval), created_at
  FROM subscriptions
ON CONFLICT DO NOTHING;

INSERT INTO product_events (user_id, name, source, dedupe_key, props, occurred_at)
SELECT user_id, 'subscription.canceled', 'backfill', ls_subscription_id,
       jsonb_build_object('plan', plan, 'interval', interval), cancelled_at
  FROM subscriptions
 WHERE cancelled_at IS NOT NULL
ON CONFLICT DO NOTHING;

INSERT INTO product_events (user_id, name, source, dedupe_key, props, occurred_at)
SELECT user_id, 'subscription.ended', 'backfill', ls_subscription_id,
       jsonb_build_object('plan', plan, 'interval', interval),
       coalesce(expired_at, last_event_at, updated_at)
  FROM subscriptions
 WHERE status = 'expired'
ON CONFLICT DO NOTHING;

INSERT INTO product_events (user_id, name, source, occurred_at)
SELECT user_id, 'export.requested', 'backfill', requested_at
  FROM export_jobs;

-- ---- the admin panel's "Funnel" tab
--
-- A function of its own, like `admin_metrics_extra` (0047): `create or replace`
-- rewrites the whole body, and nobody wants to copy 700 lines to add a number.
-- Same gate. Weeks and days in Europe/Madrid, like the others.

create or replace function public.admin_product_events()
returns jsonb
language plpgsql
security definer
set search_path = public
as $$
declare
  admin_uid constant uuid := 'f08eb80b-cbe8-4997-b69a-f2b9a5d6630a';
  tz        constant text := 'Europe/Madrid';
  out_json  jsonb;
begin
  if auth.uid() is distinct from admin_uid then
    raise exception 'not authorized' using errcode = '42501';
  end if;

  with
  ev as (
    select e.user_id, e.name, e.props, e.source, e.device_fingerprint, e.occurred_at
      from product_events e
     where e.user_id is distinct from admin_uid
  ),
  -- The second machine an account ever registered. `device.registered` has no
  -- once-key, so the order of the rows is the count.
  dev2 as (
    select user_id, occurred_at
      from (select user_id, occurred_at,
                   row_number() over (partition by user_id order by occurred_at) as n
              from ev
             where name = 'device.registered') d
     where n = 2
  ),
  -- When each account first got to each step. One row per account.
  r as (
    select x.*, dev2.occurred_at as second_device
      from (select user_id,
                   min(occurred_at) filter (where name = 'account.created')      as signed_up,
                   min(occurred_at) filter (where name = 'upload.first')         as uploaded,
                   min(occurred_at) filter (where name = 'download.first')       as downloaded,
                   min(occurred_at) filter (where name = 'sync.cross_device')    as cross_device,
                   min(occurred_at) filter (where name = 'storage.threshold'
                                              and props->>'pct' = '80')          as hit_80,
                   min(occurred_at) filter (where name = 'quota.blocked')        as blocked,
                   min(occurred_at) filter (where name = 'upsell.shown')         as upsell,
                   min(occurred_at) filter (where name = 'pricing.opened')       as pricing,
                   min(occurred_at) filter (where name = 'checkout.requested')   as checkout,
                   min(occurred_at) filter (where name = 'subscription.started') as paid
              from ev
             group by user_id) x
      left join dev2 using (user_id)
  ),
  funnel as (
    select jsonb_build_array(
             jsonb_build_object('step', 'account.created',      'users', count(signed_up)),
             jsonb_build_object('step', 'upload.first',         'users', count(uploaded)),
             jsonb_build_object('step', 'device.second',        'users', count(second_device)),
             jsonb_build_object('step', 'sync.cross_device',    'users', count(cross_device)),
             jsonb_build_object('step', 'storage.80',           'users', count(hit_80)),
             jsonb_build_object('step', 'quota.blocked',        'users', count(blocked)),
             jsonb_build_object('step', 'pricing.opened',       'users', count(pricing)),
             jsonb_build_object('step', 'checkout.requested',   'users', count(checkout)),
             jsonb_build_object('step', 'subscription.started', 'users', count(paid))
           ) as steps
      from r
  ),
  -- The same steps for each week's sign-ups, so a change shows up in the
  -- cohort it touched instead of being averaged into everyone since July.
  cohorts as (
    select date_trunc('week', signed_up at time zone tz)::date as week,
           count(*)             as signups,
           count(uploaded)      as uploaded,
           count(second_device) as second_device,
           count(cross_device)  as cross_device,
           count(hit_80)        as hit_80,
           count(blocked)       as blocked,
           count(pricing)       as pricing,
           count(checkout)      as checkout,
           count(paid)          as paid
      from r
     where signed_up > now() - interval '12 weeks'
     group by 1
  ),
  weekly as (
    select date_trunc('week', occurred_at at time zone tz)::date as week,
           name,
           count(*)                as events,
           count(distinct user_id) as users
      from ev
     where occurred_at > now() - interval '26 weeks'
       and name in ('account.created', 'active.day', 'app.opened', 'upload.first',
                    'quota.blocked', 'pricing.opened', 'checkout.requested',
                    'subscription.started', 'subscription.ended')
     group by 1, 2
  ),
  daily_active as (
    select (occurred_at at time zone tz)::date as day,
           count(distinct user_id)             as users,
           count(distinct device_fingerprint)  as devices
      from ev
     where name = 'active.day'
       and occurred_at > now() - interval '60 days'
     group by 1
  ),
  catalog as (
    select name,
           count(*) filter (where occurred_at > now() - interval '30 days')               as events_30d,
           count(distinct user_id) filter (where occurred_at > now() - interval '30 days') as users_30d,
           count(*)                                                                       as events_all,
           count(distinct user_id)                                                        as users_all,
           count(*) filter (where source = 'backfill')                                    as backfilled,
           max(occurred_at)                                                               as last_at
      from ev
     group by name
  ),
  pricing_from as (
    select coalesce(props->>'from', '(?)') as source,
           count(*)                        as events,
           count(distinct user_id)         as users
      from ev
     where name = 'pricing.opened'
       and occurred_at > now() - interval '90 days'
     group by 1
  ),
  -- Everyone who paid, and what had happened to them before they did. Four
  -- paying accounts by sep-2026 and the motive was storage every time; this is
  -- what says whether that still holds.
  conversions as (
    select p.email,
           r.signed_up,
           r.paid,
           round((extract(epoch from (r.paid - r.signed_up)) / 3600.0)::numeric, 1) as hours_to_pro,
           r.blocked is not null and r.blocked <= r.paid               as blocked_before,
           r.hit_80 is not null and r.hit_80 <= r.paid                 as hit_80_before,
           r.second_device is not null and r.second_device <= r.paid   as multi_device_before,
           r.cross_device is not null and r.cross_device <= r.paid     as synced_before,
           (select count(*) from ev
             where ev.user_id = r.user_id
               and ev.name = 'pricing.opened'
               and ev.occurred_at <= r.paid)                           as pricing_views_before
      from r
      join profiles p on p.user_id = r.user_id
     where r.paid is not null
     order by r.paid desc
     limit 50
  )
  select jsonb_build_object(
    'generated_at', now(),
    'funnel',       (select steps from funnel),
    'cohorts',      (select coalesce(jsonb_agg(row_to_json(c) order by c.week), '[]'::jsonb) from cohorts c),
    'weekly',       (select coalesce(jsonb_agg(row_to_json(w) order by w.week, w.name), '[]'::jsonb) from weekly w),
    'daily_active', (select coalesce(jsonb_agg(row_to_json(d) order by d.day), '[]'::jsonb) from daily_active d),
    'catalog',      (select coalesce(jsonb_agg(row_to_json(c) order by c.events_30d desc, c.name), '[]'::jsonb) from catalog c),
    'pricing_from', (select coalesce(jsonb_agg(row_to_json(f) order by f.events desc), '[]'::jsonb) from pricing_from f),
    'conversions',  (select coalesce(jsonb_agg(row_to_json(c)), '[]'::jsonb) from conversions c)
  ) into out_json;

  return out_json;
end;
$$;

revoke all on function public.admin_product_events() from public, anon;
grant execute on function public.admin_product_events() to authenticated;
