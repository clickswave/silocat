-- Registered users' files are private unless explicitly shared.
--
-- `public_access` is what fetch_chunks / fetch_files / fetch_resource consult to
-- serve a NON-owner who supplies only the file id. The web client used to send
-- `public_access = NOT encrypted` on every upload, so each unencrypted file in a
-- registered user's drive was downloadable by anyone holding the id, with no
-- share link ever created and no way for the owner to tell. create_files.rs now
-- forces the column to false for any upload that has a user_id, and
-- share.rs::toggle_share owns it from there. This clears the rows created before
-- that.
--
-- Scope and safety:
--   * Only rows with user_id IS NOT NULL. Anonymous drops (user_id IS NULL) are
--     public-by-link by design and are left exactly as they are.
--   * Rows the owner has actually shared keep their access. "Actually shared"
--     means a live token: share_type set and not 'off', and a token present.
--     Revoking those would break links people have already sent out, which is a
--     different and much louder kind of breakage.
--   * Anything else in a registered drive goes private.
UPDATE files
SET public_access = false
WHERE user_id IS NOT NULL
  AND public_access = true
  AND (share_type IS NULL OR share_type = 'off' OR share_token IS NULL);

-- Sharing that was explicitly switched off should not have left a live token
-- behind. Same reasoning as above: "off" has to mean the URL is dead, otherwise
-- re-enabling sharing silently resurrects a link the owner believed was gone.
UPDATE files
SET share_token = NULL, public_access = false
WHERE share_type = 'off' AND share_token IS NOT NULL;

UPDATE folders
SET share_token = NULL
WHERE share_type = 'off' AND share_token IS NOT NULL;
