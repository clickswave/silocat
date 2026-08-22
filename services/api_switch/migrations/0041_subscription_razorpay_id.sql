-- Recurring subscriptions: link a `subscriptions` row to the Razorpay
-- Subscription that drives its charges, so a `subscription.charged` webhook can
-- extend the right row's expiry and a cancellation/halt can stop it. NULL for
-- legacy one-time purchases, promo grants, and invite/trial rows.
ALTER TABLE subscriptions ADD COLUMN IF NOT EXISTS razorpay_subscription_id TEXT;
CREATE INDEX IF NOT EXISTS idx_subscriptions_rzp_sub
    ON subscriptions (razorpay_subscription_id);
