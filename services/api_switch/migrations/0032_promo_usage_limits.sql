-- Security fix (silocat-suggestions.md P0 #6): promo_codes had no usage limit or
-- expiry, so any active code could be redeemed an unlimited number of times.
-- Add a max-uses cap, a redemption counter, and an optional expiry; enforcement
-- is an atomic conditional increment at order time.
ALTER TABLE promo_codes
    ADD COLUMN max_uses   INT,                        -- NULL = unlimited
    ADD COLUMN uses_count INT NOT NULL DEFAULT 0,
    ADD COLUMN expires_at TIMESTAMPTZ;                -- NULL = no expiry
