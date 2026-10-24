-- A refunded payment takes Pro away, whatever was bought: a pass, a month, a
-- year. The row is expired on the spot and this records why, which `status`
-- alone cannot: an expired row may have simply run out.
--
-- It is also what keeps the refund in force. Cancelling a subscription in
-- Polar sends `subscription.updated` with status `active` (Polar keeps it
-- active until the period ends), and without a mark that event would hand Pro
-- straight back. Only a new paid period clears it.
ALTER TABLE public.subscriptions ADD COLUMN IF NOT EXISTS refunded_at timestamptz;
