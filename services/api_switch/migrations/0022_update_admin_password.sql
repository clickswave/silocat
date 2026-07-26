-- Historically rotated the seeded admin password hash.
--
-- Emptied deliberately: it contained a live credential. Admin access is now a
-- secret in `ADMIN_SECRET`, not an account. See 0036.
SELECT 1;
