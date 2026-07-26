-- API keys at rest: blind index + ciphertext.
--
-- The `api_key` column changes meaning from "the key" to "HMAC-SHA256 of the
-- key". `api_key_enc` holds XChaCha20-Poly1305 ciphertext so the owner can
-- still read their key back in Settings.
--
-- The values themselves cannot be converted in SQL: both need the app's
-- API_KEY_ENC_KEY. `api_keys_migrated` marks rows still holding plaintext, and
-- the one-shot backfill in libs::apikey_backfill converts them on startup.

ALTER TABLE users ADD COLUMN IF NOT EXISTS api_key_enc TEXT;
ALTER TABLE users ADD COLUMN IF NOT EXISTS api_key_migrated BOOLEAN NOT NULL DEFAULT FALSE;

ALTER TABLE anonymous_users ADD COLUMN IF NOT EXISTS api_key_migrated BOOLEAN NOT NULL DEFAULT FALSE;

-- Lookups are exact-match on a 64-char hex digest.
CREATE INDEX IF NOT EXISTS idx_users_api_key ON users (api_key);
CREATE INDEX IF NOT EXISTS idx_anonymous_users_api_key ON anonymous_users (api_key);

-- Ownership comparisons for anonymous objects match on the same value.
CREATE INDEX IF NOT EXISTS idx_files_owner_api_key ON files (owner_api_key);
CREATE INDEX IF NOT EXISTS idx_folders_owner_api_key ON folders (owner_api_key);

-- Rows that predate this migration still hold plaintext; the backfill finds
-- them by this flag rather than guessing from the value's shape.
UPDATE users SET api_key_migrated = FALSE WHERE api_key_enc IS NULL;
