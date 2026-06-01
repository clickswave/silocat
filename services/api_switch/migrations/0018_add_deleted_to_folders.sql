-- Add deleted column to folders
ALTER TABLE folders ADD COLUMN IF NOT EXISTS deleted BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE folders ADD COLUMN IF NOT EXISTS deleted_on TIMESTAMPTZ;

-- Add deleted_on column to files (files already has deleted column)
ALTER TABLE files ADD COLUMN IF NOT EXISTS deleted_on TIMESTAMPTZ;
