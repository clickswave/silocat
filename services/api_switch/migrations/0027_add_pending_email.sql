-- Self-service email change with re-verification: stage the new address + its
-- OTP until the user confirms, so the verified account email is untouched
-- until verification succeeds.
ALTER TABLE users
    ADD COLUMN pending_email     TEXT,
    ADD COLUMN pending_email_otp TEXT;
