-- Where an account came from, so the console can tell which surface earns
-- signups without depending on Google Analytics (which ad blockers and a strict
-- CSP have both kept blind before).
--
-- The web app keeps the first `ref` a visitor arrived with (a download page's
-- "send your own" link, a newsletter, a post) and passes it at signup. Free
-- text from the client, cleaned to [a-z0-9_-] and 40 characters by the API.
-- NULL for every account made before this existed.
ALTER TABLE users ADD COLUMN IF NOT EXISTS signup_source TEXT;
