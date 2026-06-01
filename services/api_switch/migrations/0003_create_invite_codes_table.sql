CREATE TABLE invite_codes
(
    -- identification
    code         TEXT        NOT NULL UNIQUE,
    description  TEXT        NOT NULL DEFAULT '',
    account_type TEXT        NOT NULL DEFAULT 'personal', -- 'personal' or 'enterprise'
    benefit     TEXT        NOT NULL DEFAULT '',         -- description of benefits associated with this code
    created_on   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    claimed_by   TEXT                 DEFAULT NULL,       -- user_id of the user who claimed the code

    -- wordlist integrity
    PRIMARY KEY (code)
);
