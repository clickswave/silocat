ALTER TABLE anonymous_users ADD COLUMN bandwidth_usage_bytes BIGINT NOT NULL DEFAULT 0;
ALTER TABLE anonymous_users ADD COLUMN last_reset_stats TIMESTAMPTZ NOT NULL DEFAULT NOW();
