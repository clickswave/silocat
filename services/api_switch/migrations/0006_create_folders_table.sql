CREATE TABLE folders (
    id TEXT PRIMARY KEY NOT NULL DEFAULT gen_random_uuid()::TEXT,
    name TEXT NOT NULL,
    user_id TEXT,
    parent_id TEXT REFERENCES folders(id) ON DELETE CASCADE,
    uploaded_as_files BOOLEAN NOT NULL DEFAULT FALSE,
    created_on TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_on TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_folders_user_id ON folders(user_id);
CREATE INDEX idx_folders_parent_id ON folders(parent_id);
