# Verifying `fix/audit-2026-08-23`

64 files, +2908 / -929, 13 commits. This is a walkthrough for checking it, ordered
by how much damage each change could do if I got it wrong. Findings themselves
are in `silo-suggests.md`; this file is only about verification.

```bash
cd projects/silocat
git log --oneline dev..HEAD
git diff dev...HEAD --stat
```

---

## First: run the checks

```bash
# 1. backend compiles the way Docker builds it (SQLX_OFFLINE, no live DB)
cd services/api_switch && SQLX_OFFLINE=true cargo check

# 2. frontend production build
cd services/web_server && SVELTEKIT_OUT_DIR=.svelte-kit-local ADAPTER=node npx vite build

# 3. types and a11y (231 errors is the expected baseline, see S3-14; 10 warnings)
npx svelte-check --tsconfig ./jsconfig.json --threshold warning

# 4. API suite: expect 66 passed
cd ../../../../scripts/silocat_testkit && python3 run.py --keep && python3 -m pytest suites/

# 5. browser suite: expect 5/5. Needs a production build on :12001, see below.
python3 -m venv .venv && .venv/bin/pip install playwright && .venv/bin/playwright install chromium
.venv/bin/python browser/run_browser_suite.py --prod
```

For step 5, port 12001 is not negotiable (R2 CORS allows exactly that origin) and
`--prod` is not optional (the dev server adds `unsafe-inline`, so it cannot test
the CSP). Full recipe in `scripts/silocat_testkit/browser/README.md`. Put the dev
container back with `docker compose -p silocat-dev -f
projects/silocat/orchestration/docker-compose.dev.yml up -d sc_web_server`, not
`docker start`, or it comes back without its port mapping.

---

## Tier 1: changes that alter who can read a file

These are the ones to read line by line. Everything else is recoverable; these
decide access.

### `create_files.rs` (1 line)

```rust
let public_access = if caller.user_id.is_some() { false } else { payload.public_access };
```

Ignores the body for any upload with a user_id. Check that anonymous drops still
get `payload.public_access`, since public-by-link is that product's whole point.

### `share.rs` `toggle_share` (~15 lines)

`share_type = 'off'` now also sets `public_access = false` and clears
`share_token`. Two things worth checking yourself:

- the folder branch deliberately does **not** set `public_access`. Folders have
  no such column and are only reachable through the token path. Confirm with
  `\d folders`.
- `token_to_set` is now `None` when unpublishing, so re-enabling issues a fresh
  token. A previously distributed URL stays dead. That is intended; say if it is
  not.

### Migration `0044` (run on dev only)

```sql
-- what it will do to production, without doing it:
SELECT count(*) FROM files
WHERE user_id IS NOT NULL AND public_access = true
  AND (share_type IS NULL OR share_type = 'off' OR share_token IS NULL);
```

Rows with a live token are deliberately left alone, so links already sent keep
working. On dev it moved 14 files and cleared 2 stale tokens; re-running the
count above now returns 0, which is also how you confirm it is idempotent.

### `respond()` in `routes/mod.rs` (1 line)

`410 => StatusCode::GONE`. There was no arm for 410, so it fell through `_ =>`
and 13 call sites sent "link expired" as **500**. Three frontend branches check
for 410 and none had ever run.

```bash
grep -rn "respond(410" services/api_switch/src | wc -l   # 13
```

### `lib/password.js` import in the Files page

The upload modal had a local `generatePassword()` built on `Math.random()`,
shadowing the CSPRNG one. Verify nothing calls a `Math.random` generator now:

```bash
grep -rn "Math.random" services/web_server/src | grep -iv "uid\|id ="
```

One hit remains, `Input.svelte` generating a DOM id, which is not a secret.

---

## Tier 2: the download rewrite

The largest single piece, and the one with the least prior art in this codebase.

**New:** `lib/fileSink.js` (three-tier sink), `src/service-worker.js` (stream
relay), `lib/cryptoClient.js` (shared worker client), `lib/chunking.js`.

**What to check:**

- `service-worker.js` handles **only** `/_stream/`. It registers no caches and
  passes everything else through, so adding it changes nothing about how the
  site loads. Read the `fetch` handler; it is 20 lines.
- `download.js` opens the sink **before** any `await`. That ordering is the whole
  reason the File System Access tier works at all; an await spends the user
  gesture and Chromium then refuses `showSaveFilePicker`. Same in
  `routes/s/[token]/+page.svelte` (`presink`).
- chunk size moved 100MB to 16MB in one shared constant. Old files are
  unaffected: boundaries are recorded per file at upload time.
- preview now refuses above 150MB, bulk zip above 2GB. Both genuinely need the
  bytes in memory; single-file downloads stream and have no limit.

**Verified:** byte-exact round trip through the service-worker tier at 2MB
(plain and encrypted) and through the sink directly at 20MB. **Not verified:**
the File System Access tier past the point of invocation, because headless has no
save dialog. Someone should download a large file in a real Chromium window, and
in Firefox and Safari, before trusting this at scale.

---

## Tier 3: the CSP

**Moved out of `_headers` into `svelte.config.js` under `kit.csp`.** This matters:
SvelteKit emits a per-request inline hydration script that cannot be hashed from
outside the framework, so a hand-written policy blocks every page. Verified: it
did exactly that.

Do not add a CSP back to `_headers`. Two policies intersect, and the site breaks.
There is a comment there saying so.

`src/app.html` no longer has an inline theme script; it is `static/theme-init.js`
and covered by `script-src 'self'`. That removes the hash-drift problem rather
than documenting it.

```bash
# what the app actually emits
curl -sI http://localhost:12001/ | grep -i content-security-policy
```

`frame-src` includes `'self'` because the download sink navigates a same-origin
iframe. Removing it silently kills downloads.

---

## Tier 4: behaviour you will notice using the app

| Change | Where | How to see it |
| --- | --- | --- |
| Folder is in the URL | `home/files/+page.svelte` | Open a folder, press Back. Reload inside a folder. Copy the URL into a new tab. |
| Copy-link publishes with an Undo | `lib/share.js` | Copy a link on an unshared file. The toast says sharing was turned on and offers Undo. |
| Files context "Copy link" | same | Previously built `/{id}`, which only worked because of the `public_access` default. Now a real `/s/<token>` link. |
| Folder zip download | same | Previously failed 100% of the time (read a `success` key no response has). |
| Folder delete count | same | Previously always said "unknown"; the proxy route did not exist. |
| API key in Settings | `home/settings` | Previously always blank. Click the eye; it fetches on demand. |
| Upload copy | Files upload modal | No longer claims "encrypted in your browser" while the toggle is off. |
| Share expiry dropdown | `ShareModal` | Shows "Keep expires <date>" instead of lying with "Never". |
| Regenerate link | `ShareModal` | Now asks first, and says how many times the link was opened. |
| Escape / focus | all dialogs | ShareModal and the two Files overlays now use `ui/Modal`. Escape closes; Tab stays inside; the page no longer scrolls behind. |
| Mobile bottom of sidebar | `home/+layout` | `100vh` to `100dvh`. Storage meter and account row were unreachable behind the URL bar. |
| Transfer guards | `lib/transferGuard.js` | Close the tab mid-upload; you get a prompt. Navigating between folders does **not** prompt. |
| Theme | `static/theme-init.js` | First visit on a light-mode OS now gets light. |

---

## Tier 5: mechanical, low risk

- **23 `console.log` removed.** Three were leaking credentials (the signup load
  printed the session including `api_key`; the signup action printed the
  FormData including the plaintext password; the upload logged presigned R2
  URLs).
- **`lib/format.js`** replaces 7 copies of `formatSize` and 2 time helpers. They
  had diverged: some produced `"undefined"` for sub-1-byte values (unclamped
  array index) and one rendered `"2 hour ago"`.
- **`lib/queryKeys.js`** replaces string literals across 5 files. Two keys were
  being invalidated that no query had ever registered (`fetchStarredFiles`,
  `fetchStarredFolders`; the Starred screen uses `starredFiles`), so starring
  never refreshed that screen. Separately the dashboard's own keys were never
  invalidated by anything, so it went stale after uploads and deletes.
- **`app.d.ts`** declares `App.Locals.session` and `Window.showSaveFilePicker`.
  That one file cleared 69 type errors across 56 others.
- **Deleted** `home/+page.server.js` and `home/files/+page.server.js`: both did a
  full file-list fetch server-side and returned `data.files`, which neither page
  read.
- **`vite.config.js`** ignores `.svelte-kit-*` and `build/` in the dev watcher. A
  host-side build was making the dev container reload-loop until it stopped
  answering.

---

## Things I want you to push back on if you disagree

1. **`public_access` forced false server-side**, ignoring the request body. It is
   the safe default and defence in depth, but it does mean an API client cannot
   create a pre-shared file in one call. Nothing does that today.

2. **Unpublishing clears the share token.** "Off" now means the URL is dead
   permanently, not dormant. Alternative is to keep it and only clear
   `public_access`.

3. **S2-17 reversed.** I wrote and tested an IP-scoped resume window for
   one-time links, then backed it out because it weakens
   `test_once_link_authorizes_exactly_once`. Chunks come from R2 by presigned
   URL, so the server never sees the transfer and "exactly once" can only mean
   "exactly one authorization". Any fix for interrupted downloads trades some of
   that away, which is your decision. ~40 lines plus two columns to reinstate.

4. **`checkJs` on with `noImplicitAny` and `useUnknownInCatchVariables` off.**
   231 errors remain, concentrated in five large components. Every file this
   branch adds is clean. The alternative is annotating ~130 files.

5. **Chunk size 100MB to 16MB.** Better progress granularity and much lower peak
   memory, at the cost of more requests per upload.

6. **Bulk zip capped at 2GB, preview at 150MB.** Both were previously unbounded
   and would OOM the tab instead.
