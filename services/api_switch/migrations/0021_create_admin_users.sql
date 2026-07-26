-- Historically created an `admin_users` table and seeded a default admin.
--
-- Emptied deliberately: the seed put a real password hash in the repository,
-- and admin access no longer uses accounts at all (see 0036 and
-- `middlewares/admin_secret.rs`). Kept as a no-op so migration numbering and
-- already-applied checksums stay meaningful.
SELECT 1;
