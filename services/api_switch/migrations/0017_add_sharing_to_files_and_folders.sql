-- Add sharing columns to files table
ALTER TABLE files
ADD COLUMN share_token TEXT,
ADD COLUMN share_type TEXT DEFAULT 'off', -- 'off', 'public', 'once'
ADD COLUMN link_downloads BIGINT DEFAULT 0,
ADD COLUMN link_max_downloads BIGINT DEFAULT 1;

ALTER TABLE files ADD CONSTRAINT unique_file_share_token UNIQUE (share_token);

-- Add sharing columns to folders table (for future proofing, even if UI comes later)
ALTER TABLE folders
ADD COLUMN share_token TEXT,
ADD COLUMN share_type TEXT DEFAULT 'off',
ADD COLUMN link_downloads BIGINT DEFAULT 0,
ADD COLUMN link_max_downloads BIGINT DEFAULT 1;

ALTER TABLE folders ADD CONSTRAINT unique_folder_share_token UNIQUE (share_token);
