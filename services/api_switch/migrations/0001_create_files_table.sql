CREATE TABLE files
(
    id              TEXT PRIMARY KEY NOT NULL DEFAULT gen_random_uuid()::TEXT,
    user_id         TEXT                      DEFAULT NULL, -- Null for anonymous uploads
    name            TEXT             NOT NULL,
    mime            TEXT             NOT NULL,              -- e.g., 'image/png', 'application/pdf'
    size            BIGINT           NOT NULL,              -- in bytes
    encrypted       BOOLEAN          NOT NULL,              -- Whether the upload is encrypted
    created_on      TIMESTAMPTZ      NOT NULL DEFAULT NOW(),

    total_chunks    INTEGER          NOT NULL,
    uploaded_chunks INTEGER          NOT NULL DEFAULT 0,
    deleted         BOOLEAN          NOT NULL DEFAULT FALSE,

    sha256_checksum TEXT             NOT NULL,              -- SHA-256 hash of the upload content
    blake3_checksum TEXT             NOT NULL,              -- BLAKE3 hash of the upload content
    downloads       BIGINT           NOT NULL DEFAULT 0,    -- Number of downloads for this file

    public_access   BOOLEAN          NOT NULL,              -- true will allow anyone to download the file with link / file id

    FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE SET NULL
);
