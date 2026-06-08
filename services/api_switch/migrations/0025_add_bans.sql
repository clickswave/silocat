-- Registered-user bans. Active ban = is_banned AND (banned_until IS NULL OR
-- banned_until > NOW()). banned_until NULL = permanent. is_restricted (existing
-- column) is the lighter "read-only / no uploads" state.
ALTER TABLE users ADD COLUMN is_banned BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE users ADD COLUMN banned_until TIMESTAMPTZ DEFAULT NULL;
ALTER TABLE users ADD COLUMN ban_reason TEXT DEFAULT NULL;

-- IP bans for anonymous (shadow) users. A row = banned; banned_until NULL =
-- permanent, otherwise active until that time. Active = banned_until IS NULL OR
-- banned_until > NOW().
CREATE TABLE ip_bans (
    id TEXT PRIMARY KEY,
    ip TEXT NOT NULL,
    reason TEXT,
    banned_until TIMESTAMPTZ DEFAULT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_ip_bans_ip ON ip_bans (ip);
