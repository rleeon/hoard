-- Server→app push prerequisite for cloud (Supabase Realtime).
--
-- The desktop client opens a Supabase Realtime WebSocket and subscribes to
-- `postgres_changes` on `public.saves` so that when *another* device commits a
-- new version the local app is kicked to pull within ~1s, instead of waiting
-- for the next poll tick. Two things have to be true in Postgres for that push
-- to arrive:
--
--   1. `public.saves` must be a member of the `supabase_realtime` publication,
--      otherwise Realtime never streams its row changes.
--   2. The authenticated role must have a SELECT RLS policy on `public.saves`,
--      because Realtime authorization replays the change through RLS as the
--      subscribing user — no SELECT policy means no event delivered.
--
-- Both already exist in the production Supabase project (they were applied by
-- hand via the dashboard), but were never versioned here. This migration makes
-- the repo the source of truth and is safe to (re)apply: every statement is
-- guarded so running it against a project that already has these does nothing.
--
-- Replica identity is left at the default (primary key). The client only needs
-- "a row in saves changed, go pull" — it never reads column values out of the
-- Realtime payload — so the PK in the change is sufficient and there is no need
-- to widen replica identity to FULL.
--
-- Postgres/Supabase only: the self-hosted server (SQLite) has no Realtime and
-- gets its own push path via the `/v1/events` SSE endpoint instead.

-- 1. Add `saves` to the Realtime publication if a) the publication exists and
--    b) the table is not already a member. `ALTER PUBLICATION … ADD TABLE`
--    errors when the table is already published, hence the membership guard.
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_publication WHERE pubname = 'supabase_realtime')
       AND NOT EXISTS (
           SELECT 1 FROM pg_publication_tables
           WHERE pubname = 'supabase_realtime'
             AND schemaname = 'public'
             AND tablename = 'saves'
       )
    THEN
        ALTER PUBLICATION supabase_realtime ADD TABLE public.saves;
    END IF;
END $$;

-- 2. SELECT policy for the owning user so Realtime authorization lets the
--    change through. Postgres has no CREATE POLICY IF NOT EXISTS, so guard on
--    pg_policies. `(select auth.uid())` is wrapped in a subselect so the
--    planner evaluates it once per statement (Supabase RLS perf guidance).
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_policies
        WHERE schemaname = 'public'
          AND tablename = 'saves'
          AND policyname = 'saves_owner_select'
    )
    THEN
        CREATE POLICY saves_owner_select ON public.saves
            FOR SELECT USING (user_id = (select auth.uid()));
    END IF;
END $$;
