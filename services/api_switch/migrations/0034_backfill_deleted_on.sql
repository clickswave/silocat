-- Trash retention needs a deletion timestamp on every trashed row.
--
-- `delete_folders` has always stamped `deleted_on`, but `delete_files` only set
-- `deleted = true`, so files already in the trash carry no deletion time. The
-- 30-day retention sweep reads `deleted_on`, and a NULL there would either be
-- skipped forever or (worse, depending on the predicate) reaped immediately.
--
-- Backfill to NOW() rather than `created_on`: these files were trashed at an
-- unknown time, and giving the user a fresh, full retention window is the safe
-- direction to be wrong in. Nothing is deleted early because of this migration.
UPDATE files
SET deleted_on = NOW()
WHERE deleted = TRUE
  AND deleted_on IS NULL;

UPDATE folders
SET deleted_on = NOW()
WHERE deleted = TRUE
  AND deleted_on IS NULL;

-- The retention sweep scans by (deleted, deleted_on); without these it degrades
-- to a sequential scan over every file once per interval.
CREATE INDEX IF NOT EXISTS idx_files_deleted_on ON files (deleted_on) WHERE deleted = TRUE;
CREATE INDEX IF NOT EXISTS idx_folders_deleted_on ON folders (deleted_on) WHERE deleted = TRUE;
