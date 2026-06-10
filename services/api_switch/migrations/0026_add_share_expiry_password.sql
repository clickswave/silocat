-- Share-link hardening: optional expiry timestamp and optional password gate.
-- Applies to both files and folders. NULL = no expiry / no password.
ALTER TABLE files
    ADD COLUMN share_expires_at    TIMESTAMPTZ,
    ADD COLUMN share_password_hash TEXT;

ALTER TABLE folders
    ADD COLUMN share_expires_at    TIMESTAMPTZ,
    ADD COLUMN share_password_hash TEXT;
