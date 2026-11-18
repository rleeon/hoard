-- Who an account belongs to at the OpenID Connect provider: the issuer and its
-- `sub` for that person, written at the first OIDC sign-in into the account.
--
-- The claim that names the account (`preferred_username`, `email`) is not
-- proof of anything on its own. A provider with self-registration lets anyone
-- pick a free name, and names get renamed and reused; `sub` is the one value
-- the spec promises never to hand to someone else. After the first sign-in,
-- a token that names this account with another `sub` is refused.
--
-- NULL = never signed in through OIDC. `hoard-admin user unlink` puts it back.
ALTER TABLE users ADD COLUMN oidc_issuer TEXT;
ALTER TABLE users ADD COLUMN oidc_subject TEXT;

-- One person at the provider, one account here.
CREATE UNIQUE INDEX IF NOT EXISTS idx_users_oidc_subject
    ON users(oidc_issuer, oidc_subject);
