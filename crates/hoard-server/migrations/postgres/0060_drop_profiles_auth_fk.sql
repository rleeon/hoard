-- profiles.user_id stops pointing at auth.users.
--
-- 0013 wired it with ON DELETE CASCADE so a hard delete in Supabase Auth took
-- the account's data along. The data now lives in a Postgres of its own, where
-- auth.users is a copy refreshed hourly (cloud::auth_mirror): a signup lands
-- in profiles before its row reaches the copy, and the constraint would refuse
-- it. Account removal does not need it either, account_purge deletes from
-- profiles and the cascade runs from there.
ALTER TABLE profiles DROP CONSTRAINT IF EXISTS profiles_user_id_fkey;
