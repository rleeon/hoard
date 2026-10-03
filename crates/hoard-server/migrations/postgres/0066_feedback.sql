-- Hoard-help: bug reports and ideas sent from inside the app.
--
-- The one feature where a self-hosted install talks to Hoard Cloud, and only
-- because the person pressed "send". So `user_id` is optional: a self-hoster
-- has no account here, and a Cloud user whose session is broken (which is
-- often why they are writing) still gets through as anonymous.
--
-- No foreign key to `profiles`, on purpose. Reports are kept 90 days and then
-- deleted with their files (`cloud::feedback::sweep`); a cascade from an account
-- purge would drop the rows and strand the R2 objects under `feedback/`, which
-- nothing else ever lists.
--
-- Lifecycle: a report is created `pending` with its file list declared, the
-- files arrive one PUT each, and `completed_at` is set once all of them are in.
-- Only completed reports reach Discord or the admin panel. A pending report
-- older than a day is an upload that died, and the sweep removes it.

CREATE TABLE IF NOT EXISTS feedback_reports (
    id            UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id       UUID,
    -- The account's address, so a Cloud user can be answered without asking.
    account_email TEXT,
    kind          TEXT        NOT NULL CHECK (kind IN ('bug', 'idea')),
    message       TEXT        NOT NULL,
    -- Whatever the person typed in the optional contact box.
    contact       TEXT,
    app_version   TEXT,
    os            TEXT,
    arch          TEXT,
    mode          TEXT        CHECK (mode IN ('cloud', 'selfhosted', 'none')),
    -- sha256 of the throttle bucket (an IPv4, or an IPv6 /64). Only the hourly
    -- and daily limits read it, so the sweep blanks it after a day.
    ip_hash       TEXT,
    -- sha256 of the token that lets the sender upload this report's files.
    upload_token  TEXT,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    completed_at  TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_feedback_reports_created
    ON feedback_reports(created_at);
CREATE INDEX IF NOT EXISTS idx_feedback_reports_ip
    ON feedback_reports(ip_hash, created_at) WHERE ip_hash IS NOT NULL;

CREATE TABLE IF NOT EXISTS feedback_files (
    report_id   UUID        NOT NULL REFERENCES feedback_reports(id) ON DELETE CASCADE,
    idx         SMALLINT    NOT NULL,
    name        TEXT        NOT NULL,
    size_bytes  BIGINT      NOT NULL,
    r2_key      TEXT        NOT NULL,
    uploaded_at TIMESTAMPTZ,
    PRIMARY KEY (report_id, idx)
);

-- Nobody reads these through a PostgREST front end, not even their author:
-- RLS on with no policy denies every role but the owner, which is the server.
ALTER TABLE feedback_reports ENABLE ROW LEVEL SECURITY;
ALTER TABLE feedback_files ENABLE ROW LEVEL SECURITY;

-- The admin panel's Hoard-help tab. Same gate as the other admin functions.
create or replace function public.admin_feedback()
returns jsonb
language plpgsql
security definer
set search_path = public
as $$
declare
  admin_uid constant uuid := 'f08eb80b-cbe8-4997-b69a-f2b9a5d6630a';
  out_json  jsonb;
begin
  if auth.uid() is distinct from admin_uid then
    raise exception 'not authorized' using errcode = '42501';
  end if;

  select jsonb_build_object(
    'generated_at', now(),
    'counts', jsonb_build_object(
      'bugs_7d',  (select count(*) from feedback_reports
                    where completed_at is not null and kind = 'bug'
                      and created_at > now() - interval '7 days'),
      'ideas_7d', (select count(*) from feedback_reports
                    where completed_at is not null and kind = 'idea'
                      and created_at > now() - interval '7 days'),
      'total',    (select count(*) from feedback_reports where completed_at is not null),
      'pending',  (select count(*) from feedback_reports where completed_at is null),
      'bytes',    (select coalesce(sum(f.size_bytes), 0)::bigint
                     from feedback_files f
                     join feedback_reports r on r.id = f.report_id
                    where r.completed_at is not null)
    ),
    'reports', coalesce((
      select jsonb_agg(row_to_json(x) order by x.created_at desc)
        from (
          select r.id, r.kind, r.message, r.contact, r.account_email,
                 r.user_id is not null as has_account,
                 r.app_version, r.os, r.arch, r.mode, r.created_at,
                 coalesce((
                   select jsonb_agg(jsonb_build_object(
                            'idx', f.idx, 'name', f.name, 'size', f.size_bytes)
                          order by f.idx)
                     from feedback_files f
                    where f.report_id = r.id
                 ), '[]'::jsonb) as files
            from feedback_reports r
           where r.completed_at is not null
           order by r.created_at desc
           limit 300
        ) x
    ), '[]'::jsonb)
  ) into out_json;

  return out_json;
end;
$$;

revoke all on function public.admin_feedback() from public, anon;
grant execute on function public.admin_feedback() to authenticated;
