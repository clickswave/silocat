-- Password reset gets its own code, apart from the email verification code in
-- `otp`. Sharing one column meant reset guesses, which anyone who knows an
-- address can send, spent the verification code's attempts and daily budget,
-- and a resent verification code replaced a reset code the owner was using to
-- claim an account someone else had registered. The reset code is long enough
-- that it needs no daily cap, so there is no budget left for a stranger to use
-- up. It has its own send cooldown for the same reason.
ALTER TABLE users
    ADD COLUMN IF NOT EXISTS reset_code         TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS reset_expires_at   TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS reset_attempts     INT NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS reset_last_sent_at TIMESTAMPTZ;
