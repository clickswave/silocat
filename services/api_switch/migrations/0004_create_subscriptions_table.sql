CREATE TABLE subscriptions
(
    id               TEXT PRIMARY KEY     DEFAULT gen_random_uuid()::TEXT,
    name             TEXT        NOT NULL,
    additional_space BIGINT      NOT NULL DEFAULT 0, -- in GB
    created_on       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by       TEXT        NOT NULL,
    expires_on       TIMESTAMPTZ NOT NULL DEFAULT NOW()
);