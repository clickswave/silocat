-- "Request a file": the inbound half of secure delivery. An account owner
-- creates a request link; anyone with the link can upload files that land in the
-- owner's account (counting against the owner's quota). The uploaded file is a
-- normal `files` row owned by the requester; `request_uploads` ties it to the
-- request, so no existing table or model changes and the live upload path is
-- untouched (requests use their own isolated handlers).
CREATE TABLE IF NOT EXISTS file_requests (
    id            TEXT PRIMARY KEY DEFAULT (gen_random_uuid())::text,
    token         TEXT NOT NULL UNIQUE,
    owner_user_id TEXT NOT NULL,        -- requests are an account feature
    label         TEXT,                 -- what the owner is asking for
    message       TEXT,                 -- optional note shown to the uploader
    folder_id     TEXT,                 -- optional destination folder
    max_uploads   INTEGER,              -- optional cap (NULL = unlimited)
    upload_count  INTEGER NOT NULL DEFAULT 0,
    expires_at    TIMESTAMPTZ,          -- optional expiry
    active        BOOLEAN NOT NULL DEFAULT TRUE,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_file_requests_owner ON file_requests (owner_user_id, created_at DESC);

CREATE TABLE IF NOT EXISTS request_uploads (
    request_id    TEXT NOT NULL,
    file_id       TEXT NOT NULL,
    uploader_name TEXT,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (request_id, file_id)
);
CREATE INDEX IF NOT EXISTS idx_request_uploads_req ON request_uploads (request_id, created_at DESC);
