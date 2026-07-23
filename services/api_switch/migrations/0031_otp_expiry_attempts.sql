-- Security fix (silocat-suggestions.md P0 #8): the account OTP is a 6-digit code
-- verified with no expiry and no attempt limit, so /user/reset-password could be
-- brute-forced into an account takeover. Add an expiry and an attempt counter so
-- a code is only briefly valid and locks out after a few wrong guesses.
ALTER TABLE users
    ADD COLUMN otp_expires_at TIMESTAMPTZ,
    ADD COLUMN otp_attempts    INT NOT NULL DEFAULT 0;
