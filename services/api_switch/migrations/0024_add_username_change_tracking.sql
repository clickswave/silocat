-- Track username changes for the "max 2 per rolling 30 days" limit.
-- window_start anchors the current 30-day window (the first change in it);
-- count is how many changes have happened since then.
ALTER TABLE users ADD COLUMN username_change_count INT NOT NULL DEFAULT 0;
ALTER TABLE users ADD COLUMN username_change_window_start TIMESTAMPTZ DEFAULT NULL;
