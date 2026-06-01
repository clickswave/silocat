CREATE TABLE IF NOT EXISTS early_access_requests (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email VARCHAR NOT NULL UNIQUE,
    status VARCHAR DEFAULT 'pending',
    created_on TIMESTAMPTZ DEFAULT NOW()
);
