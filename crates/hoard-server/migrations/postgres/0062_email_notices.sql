-- One row per notice a user has been emailed, and when.
--
-- The point is "not once per retry". The paths that trigger these notices are
-- hot: `maybe_purge` runs on every upload, and the 402 that rejects an upload
-- is re-emitted every time the client tries again. Keying the send on a row
-- here is what turns a loop into a message.
--
-- `scope` is what the notice is *about* when it is not about the whole
-- account: a save id for "this game is too big" and for "this archived game
-- expires", empty for account-wide ones. Two oversized games should produce
-- two emails; two backup attempts of the same game should not.
--
-- Two kinds of cadence share the table:
--
--   * once, until something clears the row (`notices::claim`): the account is
--     full, a game is over the cap, every device slot is taken.
--   * once a day while a condition persists (`notices::claim_periodic`): the
--     purge is eating history, which is worth repeating precisely because it
--     keeps happening. `muted_at` is how the reader says "enough"; the daily
--     sweep drops rows nothing has refreshed in a fortnight, so a mute expires
--     on its own once the situation stops.
--
-- `mute_token` is what the link in the email carries. Random per row, so it
-- reveals no user id, and only ever mutes the one notice it belongs to.

CREATE TABLE IF NOT EXISTS email_notices (
    user_id    UUID        NOT NULL REFERENCES profiles(user_id) ON DELETE CASCADE,
    kind       TEXT        NOT NULL,
    scope      TEXT        NOT NULL DEFAULT '',
    sent_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    muted_at   TIMESTAMPTZ,
    mute_token UUID        NOT NULL DEFAULT gen_random_uuid(),
    PRIMARY KEY (user_id, kind, scope)
);

-- Deleting a save has to clear its per-save notices, and account purge cascades
-- from profiles, so the only lookup that is not already the primary key is
-- "every notice about this scope".
CREATE INDEX IF NOT EXISTS idx_email_notices_scope
    ON email_notices(scope) WHERE scope <> '';

-- The mute link resolves a row by token alone.
CREATE UNIQUE INDEX IF NOT EXISTS idx_email_notices_mute_token
    ON email_notices(mute_token);

-- Every per-user table in this schema is locked down, and the two migrations
-- that had to go back and do it (0055, 0056) are why this is here from the
-- start: rebuilt against a PostgREST front end, an unprotected table would let
-- any authenticated user read who is out of space and which of their games are
-- about to be deleted. The server reaches this table as the owner and is
-- unaffected.
ALTER TABLE email_notices ENABLE ROW LEVEL SECURITY;

DROP POLICY IF EXISTS email_notices_self ON email_notices;
CREATE POLICY email_notices_self ON email_notices
    FOR ALL
    USING ((SELECT auth.uid()) = user_id)
    WITH CHECK ((SELECT auth.uid()) = user_id);
