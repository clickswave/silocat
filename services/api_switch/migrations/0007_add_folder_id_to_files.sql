ALTER TABLE files ADD COLUMN folder_id TEXT REFERENCES folders(id) ON DELETE CASCADE;
CREATE INDEX idx_files_folder_id ON files(folder_id);
