CREATE TABLE chunks
(
    id             TEXT PRIMARY KEY NOT NULL DEFAULT gen_random_uuid()::TEXT,
    file_id        TEXT             NOT NULL,               -- Reference to files.id
    chunk_index    INTEGER          NOT NULL,               -- Position in sequence, starts at 0
    size           BIGINT           NOT NULL,               -- Size of this chunk in bytes
    size_on_server BIGINT           NOT NULL,               -- Size of this chunk on disk (after compression, if applicable)
    uploaded       BOOLEAN          NOT NULL DEFAULT FALSE, -- Upload status
    uploading      BOOLEAN          NOT NULL DEFAULT FALSE, -- Upload status
    created_on     TIMESTAMPTZ      NOT NULL DEFAULT NOW(),

    presigned_url  TEXT             NOT NULL,               -- Silo ID for distributed storage
    file_offset    BIGINT           NOT NULL,               -- Offset in the upload for this chunk
    downloads      BIGINT           NOT NULL DEFAULT 0,     -- Number of downloads for this chunk

    salt           TEXT                      DEFAULT NULL,  -- Salt for chunk integrity
    nonce          TEXT                      DEFAULT NULL,  -- Nonce for chunk integrity
    checksum       TEXT             NOT NULL,               -- Checksum for chunk integrity

    -- Ensure each chunk for a upload has unique position
    UNIQUE (file_id, chunk_index),

    FOREIGN KEY (file_id) REFERENCES files (id) ON DELETE CASCADE
);
