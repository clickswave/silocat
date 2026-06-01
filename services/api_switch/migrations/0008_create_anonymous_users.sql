CREATE TABLE anonymous_users (
    api_key TEXT PRIMARY KEY,
    ip_address TEXT NOT NULL,
    user_agent TEXT,
    geo_location JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_seen TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
