-- Opting out of the offers inside service emails, without opting out of the
-- emails.
--
-- Every notice ends in a pitch for Pro, and that part is a commercial
-- communication. Spanish law (LSSI art. 21) allows sending one to an existing
-- customer about a similar service of our own only if each message offers a
-- free, simple way to refuse it. The notice itself is a service message and
-- keeps coming; what the reader turns off is the pitch.
--
-- `offers_opt_out_at` rather than a boolean: when somebody said no is what we
-- would have to show if anyone ever asked whether we honoured it.
--
-- `offers_token` is what the link in the footer carries. Random per profile,
-- so the link identifies nobody to whoever it is forwarded to. The volatile
-- default makes Postgres rewrite the table to give each existing row its own
-- value; `profiles` is a few hundred rows, so that is a blink.

ALTER TABLE profiles
    ADD COLUMN IF NOT EXISTS offers_opt_out_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS offers_token      UUID NOT NULL DEFAULT gen_random_uuid();

CREATE UNIQUE INDEX IF NOT EXISTS idx_profiles_offers_token ON profiles(offers_token);
