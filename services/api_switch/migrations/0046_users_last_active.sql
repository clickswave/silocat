-- When an account last did anything, for the admin console's "inactive 30d"
-- view and the last-active column on every silo.cat account.
--
-- Written by libs::activity::touch from the two auth middlewares, at most once
-- every five minutes per account, so a busy session costs one UPDATE per five
-- minutes rather than one per request. Accounts that predate the column start
-- from their newest upload, or their signup when they never uploaded.
ALTER TABLE users ADD COLUMN IF NOT EXISTS last_active_at TIMESTAMPTZ;

UPDATE users u
SET last_active_at = GREATEST(
    u.created_on,
    (SELECT MAX(f.created_on) FROM files f WHERE f.user_id = u.id)
)
WHERE u.last_active_at IS NULL;

-- The console aggregates usage, links and subscriptions per account, and
-- nothing indexed files or subscriptions by owner until now.
CREATE INDEX IF NOT EXISTS idx_files_user_id ON files(user_id);
CREATE INDEX IF NOT EXISTS idx_subscriptions_created_by ON subscriptions(created_by);
