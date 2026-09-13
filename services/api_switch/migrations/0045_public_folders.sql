-- Folders can be published, the same way files can.
--
-- `public_access` is what fetch_chunks / fetch_files / fetch_resource consult to
-- serve a NON-owner holding only the resource id. `files` has carried it since
-- 0001. `folders` never did, so the folder branch of fetch_resource had no
-- column to consult and answered every non-owner with 404.
--
-- That turned every anonymous folder drop into a link only its uploader could
-- open. The upload page hands out /<folder_id> for any folder or multi-file
-- upload, so those links have been dead for their recipients since the owner
-- check landed on 2026-08-22. Single-file drops were unaffected, which is why it
-- went unnoticed: the common path still worked.
ALTER TABLE folders ADD COLUMN public_access BOOLEAN NOT NULL DEFAULT false;

-- An anonymous drop is public by link, by design. It has no drive to sit in and
-- no owner who can log in later to publish it, so the id IS the capability. This
-- is the rule create_files.rs already applies to an anonymous file, and applying
-- it here revives the drop links that are already in people's hands.
UPDATE folders
SET public_access = true
WHERE user_id IS NULL
  AND deleted = false;

-- A registered user's folder is published by the share sheet and nothing else,
-- so this mirrors what toggle_share writes from now on: a live token means
-- published. Folders with sharing off, or never shared, stay private.
UPDATE folders
SET public_access = true
WHERE user_id IS NOT NULL
  AND deleted = false
  AND share_type IS NOT NULL
  AND share_type <> 'off'
  AND share_token IS NOT NULL;
