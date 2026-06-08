-- Signup promo codes: grant bonus storage to up to N users for a duration.
-- Distinct from promo_codes (which are billing discounts). Redeemed optionally
-- at registration; the storage grant is recorded as a subscription row so it
-- expires automatically (or is effectively indefinite).
CREATE TABLE signup_promos (
    code          TEXT PRIMARY KEY,
    description   TEXT        NOT NULL DEFAULT '',
    bonus_bytes   BIGINT      NOT NULL DEFAULT 0,   -- extra storage granted on redemption
    duration_days INTEGER,                          -- NULL = indefinite
    max_uses      INTEGER,                          -- NULL = unlimited
    uses_count    INTEGER     NOT NULL DEFAULT 0,
    active        BOOLEAN     NOT NULL DEFAULT TRUE,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
