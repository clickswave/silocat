CREATE TABLE users
(
    id                    TEXT PRIMARY KEY NOT NULL DEFAULT gen_random_uuid()::TEXT,
    username              TEXT             NOT NULL UNIQUE,
    email                 TEXT             NOT NULL UNIQUE,
    password_hash         TEXT             NOT NULL,
    api_key               TEXT             NOT NULL,
    created_on            TIMESTAMPTZ      NOT NULL DEFAULT NOW(),
    account_type          TEXT             NOT NULL,           -- 'personal', 'enterprise'

    profile_image         TEXT                      DEFAULT NULL,
    email_verified        BOOLEAN          NOT NULL DEFAULT FALSE,
    otp                   TEXT             NOT NULL,

    sessions              TEXT[]           NOT NULL DEFAULT '{}'::TEXT[],
    is_restricted         BOOLEAN          NOT NULL DEFAULT FALSE,

    team_id               TEXT,
    subscription_id       TEXT,

    default_storage_bytes BIGINT           NOT NULL DEFAULT 0, -- Total storage available for the user

    transactions          JSONB[]          NOT NULL DEFAULT '{}'::JSONB[]
);