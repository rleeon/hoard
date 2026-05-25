-- Lemon Squeezy products vs variants.
--
-- Originally we keyed `subscriptions.ls_variant_id` on the LS variant_id
-- because the first draft used a single product with two variants
-- (monthly + yearly). The actual store, however, uses two *separate*
-- products — Pro Monthly and Pro Yearly — each with its own product_id.
-- The webhook attributes carry both `product_id` and `variant_id`, but
-- our config maps by product_id, so the column rename keeps schema +
-- code aligned.
--
-- Existing rows (if any) carry the old variant_id; that's fine — they
-- will be superseded by future webhooks now that we resolve by
-- product_id. The column type is unchanged.

ALTER TABLE public.subscriptions
    RENAME COLUMN ls_variant_id TO ls_product_id;
