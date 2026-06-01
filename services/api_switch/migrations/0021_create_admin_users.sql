CREATE TABLE admin_users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    role TEXT NOT NULL DEFAULT 'superadmin',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Default admin: admin@silo.cat / password
-- Hash generated with argon2id (m=4096, t=3, p=1)
INSERT INTO admin_users (email, password_hash) 
VALUES ('admin@silo.cat', '$argon2id$v=19$m=4096,t=3,p=1$c2FsdHNhbHQ$aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa');
