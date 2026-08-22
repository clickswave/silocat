-- Delivery receipts: record each time a shared link is authorized for download,
-- so the owner has proof of who accessed a deliverable and when (the core of the
-- "secure delivery" positioning). Owner-scoped reads; anonymous accessors are
-- identified only by IP + user agent, which the download page already exposes to
-- the server. Rows are pruned with their share when the file is hard-deleted (no
-- FK so a soft-delete keeps the history; the trash GC path can clear them later).
CREATE TABLE IF NOT EXISTS share_access_events (
    id            TEXT PRIMARY KEY DEFAULT (gen_random_uuid())::text,
    share_token   TEXT NOT NULL,
    file_id       TEXT,
    owner_user_id TEXT,
    ip            TEXT,
    user_agent    TEXT,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_share_access_owner ON share_access_events (owner_user_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_share_access_token ON share_access_events (share_token, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_share_access_file  ON share_access_events (file_id, created_at DESC);
