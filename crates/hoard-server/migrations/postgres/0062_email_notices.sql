-- One row per notice a user has already been emailed.
--
-- The point is "send this once, not once per retry". The paths that trigger
-- these notices are hot: `maybe_purge` runs on every upload, and the 402 that
-- means "you are full" is re-emitted every time the client tries again. Keying
-- the send on a row here turns each of them into a single message.
--
-- `scope` is what the notice is *about* when it is not about the whole
-- account: a save id for "this game is too big" and for "this archived game
-- expires", empty for account-wide ones. Two oversized games should produce
-- two emails; two backup attempts of the same game should not.
--
-- Rearming is a DELETE, done where the condition stops being true (dropping
-- back under the purge threshold, freeing a device slot, reactivating an
-- archived save). That is deliberate: a user who fills up, clears space and
-- fills up again next month gets told again.

CREATE TABLE IF NOT EXISTS email_notices (
    user_id UUID        NOT NULL REFERENCES profiles(user_id) ON DELETE CASCADE,
    kind    TEXT        NOT NULL,
    scope   TEXT        NOT NULL DEFAULT '',
    sent_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, kind, scope)
);

-- Deleting a save has to clear its per-save notices, and account purge cascades
-- from profiles, so the only lookup that is not already the primary key is
-- "every notice about this scope".
CREATE INDEX IF NOT EXISTS idx_email_notices_scope
    ON email_notices(scope) WHERE scope <> '';
