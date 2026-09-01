-- Two things the bell panel couldn't express: more than one button, and a
-- broadcast meant for a single account.
--
-- ## `actions`
--
-- `action_url`/`action_label` carry exactly one call to action, rendered as a
-- text link. A message that wants to offer two ("star the repo" / "sponsor
-- it") had no way to say so, and the body can't stand in: the client escapes
-- every byte of it before formatting (`renderMarkdown`), which is what keeps
-- the database from being able to inject markup into the app.
--
-- JSONB rather than `action2_*` columns: the next message that wants three
-- buttons shouldn't need another migration. Shape is a list of
--   {"url": "...", "label": "...", "icon": "star"}
-- with `icon` an OPTIONAL name from a fixed client-side set, never markup:
-- the server says which icon, the client owns what it looks like. Unknown
-- names render as a plain button, so a new icon name can ship server-side
-- before the client that draws it.
--
-- The old columns stay: broadcasts already sent still use them, and the list
-- handler falls back to them when `actions` is NULL.
--
-- ## `audience_user_id`
--
-- Migration 0032 deliberately gave this table no user column, so broadcasts
-- by construction. That is still the rule for everything the operator sends to
-- the world, and NULL (the default, and every existing row) keeps meaning
-- exactly that.
--
-- What it could not do is send a message to ONE account, and the case that
-- needs it is testing: seeing a new broadcast rendered in the real panel,
-- with its real buttons, without showing a half-finished message to every
-- user. A nullable pointer buys that without weakening the guarantee: a row
-- can be for everyone or for one person, never for a segment.
ALTER TABLE notifications
    ADD COLUMN IF NOT EXISTS actions JSONB,
    ADD COLUMN IF NOT EXISTS audience_user_id UUID REFERENCES profiles(user_id) ON DELETE CASCADE;

-- `actions` must be a LIST of objects when present. Without this a typo'd
-- insert (an object instead of an array) would reach clients and render
-- nothing, silently.
ALTER TABLE notifications
    DROP CONSTRAINT IF EXISTS notifications_actions_is_array;
ALTER TABLE notifications
    ADD CONSTRAINT notifications_actions_is_array
    CHECK (actions IS NULL OR jsonb_typeof(actions) = 'array');

-- Targeted notifications are looked up by recipient; broadcasts (the vast
-- majority) stay out of the index.
CREATE INDEX IF NOT EXISTS idx_notifications_audience
    ON notifications(audience_user_id)
    WHERE audience_user_id IS NOT NULL;
