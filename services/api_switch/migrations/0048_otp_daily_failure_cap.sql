-- The per-code attempt counter (0031) starts again at zero with every new code,
-- and anyone who knows an address can have a new reset code issued every 60
-- seconds, so five guesses a minute against /user/reset-password never ran out.
-- Count wrong codes per account over a 24 hour window as well, a budget that a
-- new code does not refill.
ALTER TABLE users
    ADD COLUMN IF NOT EXISTS otp_day_failures   INT NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS otp_day_started_at TIMESTAMPTZ;
