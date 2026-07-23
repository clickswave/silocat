-- Security fix (silocat-suggestions.md P0 #6): migration 0015 seeded active
-- discount codes, including a 100%-off code ('100-off-pro-1m'), and promo_codes
-- has no usage limit / expiry — so these grant unlimited free or discounted Pro.
-- Deactivate every seeded code. Real promo codes must be created out-of-band
-- with usage limits (see the follow-up that adds max_uses/uses_count/expires_at).
UPDATE promo_codes
SET active = false
WHERE code IN ('10-off-pro-1m', '15-off-pro-1m', '25-off-pro-1m', '100-off-pro-1m');
