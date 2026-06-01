CREATE TABLE orders
(
    -- user credentials and profile
    user_id            TEXT        NOT NULL,
    created_on         TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    subscription_name  TEXT        NOT NULL,
    subscription_cycle TEXT        NOT NULL,
    additional_space   BIGINT      NOT NULL DEFAULT 0,         -- in GB

    reference_id       TEXT        NOT NULL UNIQUE,
    payment_gateway    TEXT        NOT NULL,

    currency           TEXT        NOT NULL,
    amount             BIGINT      NOT NULL,                   -- in smallest unit of the currency
    status             TEXT        NOT NULL DEFAULT 'pending', -- pending, completed, failed

    details            JSONB       NOT NULL,
    transactions       JSONB[]     NOT NULL DEFAULT '{}'::JSONB[],
    PRIMARY KEY (reference_id)
);
