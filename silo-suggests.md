# Silocat: UI/UX and security review

> **Status: all findings resolved on branch `fix/audit-2026-08-23`** (2026-08-23).
> Eight commits, verified by `cargo check`, a full production `vite build`, and
> the silocat testkit (66 passed). `svelte-check` went from 894 errors / 27
> warnings to 0 errors / 10 warnings. Nothing was deployed.
>
> Two claims below were wrong and are corrected in place, marked **[correction]**:
> the resend-verification aside in S2-2, and an assumption about `/folder/stats`
> in S1-8.
>
> Deliberately not fixed, with reasons recorded inline: S2-17 (one-time links
> still burn at authorization, because the alternative is worse) and the ten
> remaining `state_referenced_locally` warnings, which are intentional
> seed-from-prop patterns.

Scope: `services/web_server` (SvelteKit) with cross-checks into `services/api_switch` (axum) where the frontend's behaviour depends on backend authorization.

Everything below was read in the source and traced end to end. Findings marked **Confirmed** have the contradicting code on both sides quoted or line-referenced. Findings marked **Risk** are real but depend on deployment conditions.

Severity legend: `S1` ship-blocker, `S2` fix soon, `S3` polish/suggestion.

---

## 1. Security

### S1-1. Encrypted uploads from the logged-in app use `Math.random()` for the password. **Confirmed**

`services/web_server/src/routes/home/files/+page.svelte:761`

```js
function generatePassword() {
    const chars = 'ABC...xyz0123456789!@#$%^&*';
    let pass = '';
    for (let i = 0; i < 16; i++) pass += chars.charAt(Math.floor(Math.random() * chars.length));
    return pass;
}
```

This local function shadows the properly built generator in `src/lib/password.js`, which uses `crypto.getRandomValues` with rejection sampling and documents its entropy. It is wired to the "Generate" button in the upload modal at `routes/home/files/+page.svelte:2479`, so **every logged-in user who taps Generate gets a `Math.random()` password**, which is a seeded PRNG whose state is recoverable from a handful of outputs. That password is the only input to `deriveKeyFromPassword`, so it is the entire security of the file.

The anonymous landing page does the right thing (`routes/+page.svelte:9` imports from `$lib/password.js`). Only the paying, logged-in path is broken.

**Fix:** delete the local function, `import { generatePassword } from '$lib/password.js'`. Same for `routes/[slug]/+page.svelte:326` (`generateRandomString`), which has the identical flaw.

### S1-2. Unencrypted uploads to a private drive are marked world-readable. **Confirmed**

`routes/home/files/+page.svelte:893` and `routes/+page.svelte:456`:

```js
public_access: !encryptionEnabled,
```

Backend, `api_switch/src/routes/file/fetch_chunks.rs:57-62`:

```rust
let allowed = caller.as_ref().map_or(false, |c| c.owns(&file.user_id, &file.owner_api_key))
    || (file.public_access && !share_protected);
```

The same clause exists in `fetch_files.rs:46` and `fetch_resource.rs:43`. So any unencrypted file a signed-in user drops into their Sanctum drive is downloadable by an unauthenticated caller who knows the file id, with no share link ever created. Ids are UUIDv4 (`libs/rng.rs`) so they are not guessable, but this is still a capability leak by id: it survives referrer leaks, log exposure, support tickets, and browser history sync, and it directly contradicts what the drive UI promises.

It also corrupts the onboarding checklist: `routes/home/+page.svelte:104` marks "Share your first link" done when `f.public_access` is true, so that step self-completes on the first plain upload without the user ever sharing anything.

**Fix:** `public_access` should default to `false` for `storage_type: 'sanctum'` and only ever be set by an explicit share action. The anonymous drop path is a different product and can keep its current default.

### S1-3. Turning sharing off does not revoke access. **Confirmed**

`api_switch/src/routes/file/share.rs:165-171`, the `toggle_share` UPDATE:

```sql
UPDATE files SET share_type = $1, share_token = $2,
  share_expires_at = CASE WHEN $3 THEN $4 ELSE share_expires_at END,
  share_password_hash = CASE WHEN $5 THEN $6 ELSE share_password_hash END
WHERE id = $7 AND user_id = $8
```

`public_access` is never touched. The token path correctly filters `share_type != 'off'` (line 515, 564, 763, 861), so the `/s/<token>` link does die. But the file keeps `public_access = true` from upload, so `fetch_chunks` and `fetch_resource` keep handing out presigned download URLs by id. A user who clicks "off" reasonably believes access is revoked. It is not.

**Fix:** set `public_access = false` in the same statement when `share_type = 'off'`, and clear `share_token` so a regenerated link cannot collide with a previously distributed one.

### S1-4. `copyShareLink` silently makes a private file public with no confirmation. **Confirmed**

`src/lib/share.js:29-52`. Copy-link buttons sit inline on cards, rows and dashboard recents. If the item is not shared, the helper posts `share_type: 'public'` on the user's behalf and then toasts "Link copied. Anyone with it can download the file."

The state change happens before the user is told what it means, and there is no undo affordance in the toast. On a touch device an accidental tap on the row action publishes a file. The comment in the file acknowledges this is deliberate ("copy link has to work in one click"), but one click to publish is different from one click to copy.

**Fix:** keep the one-click flow but make the toast carry an "Undo" action that posts `share_type: 'off'` (and, per S1-3, clears `public_access`). At minimum change the copy to past-tense state ("Sharing turned on. Anyone with this link can download it.") so the user knows a change was made.

### S1-5. Credentials and plaintext passwords are written to server logs. **Confirmed**

- `routes/auth/signup/+page.server.js:17` -> `console.log({ session })`. The session JWT payload carries `user.api_key`, which is the long-lived credential for the entire programmatic surface. This fires on every visit to `/auth/signup` by a logged-in user.
- `routes/auth/signup/+page.server.js:47` -> `console.log({SIGNUP_RESPONSE: {data, success}})` where `data` is the request `FormData`, which contains the plaintext `password` field.
- `routes/api/v1/billing/order/+server.js:21` -> `console.log("response", response)` on the Razorpay order object.

On Cloudflare Pages these land in Workers logs and any connected log drain. This is the more serious sibling of the `_headers` CSP gap: the app spends real effort keeping `api_key` out of the hydration payload (`routes/+layout.server.js:57-66`, with a good comment explaining why) and then prints it to stdout.

**Fix:** delete all three. See S3-1 for the rest of the debug logging.

### S1-6. Presigned R2 URLs are logged to the browser console. **Confirmed**

`routes/home/files/+page.svelte:920`

```js
console.log(`[Upload] Chunk ${i} URL:`, serverChunk.presigned_url);
```

A presigned URL is a bearer capability for that chunk. Console output is readable by any browser extension with `activeTab`, and by any client error reporter. Combined with the missing `script-src` CSP this is a cheap exfiltration channel.

### S2-1. No `script-src` / `connect-src` CSP. **Confirmed, acknowledged in-repo**

`services/web_server/_headers` enforces only `frame-ancestors 'none'` and says so in its own comment. The app runs `libsodium` WASM, blob workers, Turnstile, Razorpay checkout, and inline theme bootstrap script in `app.html`, so a strict policy needs staging. But the current state means any injected script runs with full access to the anonymous `shadow_api_key` in `localStorage` (`src/lib/stores/shadow.js:7`) and to decrypted plaintext held in memory during download.

**Suggested starting policy** to validate on staging:

```
Content-Security-Policy: default-src 'self'; script-src 'self' 'wasm-unsafe-eval' 'sha256-<hash-of-app.html-inline-theme-script>' https://challenges.cloudflare.com https://checkout.razorpay.com; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob: https:; font-src 'self'; worker-src 'self' blob:; connect-src 'self' https://*.r2.cloudflarestorage.com https://api.silo.cat https://challenges.cloudflare.com https://lumberjack.razorpay.com; frame-src https://challenges.cloudflare.com https://api.razorpay.com; frame-ancestors 'none'; base-uri 'none'; form-action 'self'
```

Hash the inline theme script rather than adding a nonce, since Pages serves the shell statically.

### S2-2. The public file-request upload endpoints have no rate limit and no captcha. **Confirmed**

`api_switch/src/routes/file/request.rs` exposes `public_info`, `upload_create` and `upload_mark_complete` with no `rate_limiter.check` call anywhere in the file. `routes/req/[token]/+page.svelte` has no Turnstile widget (Turnstile appears only on signin and signup). Anyone with a request link can loop uploads into the owner's quota.

Same gap in `src/routes/validate_shadow_user.rs`, which mints an anonymous user row per browser-supplied key, so a loop can farm identities and the storage grants attached to them.

**[correction]** This finding originally also named `src/routes/user/resend_verification.rs`. That was wrong. The handler requires an authenticated `UserTokenData` and enforces a 60-second cooldown from `otp_last_sent_at`, so the only inbox anyone can flood is their own. It throttles with a database timestamp rather than the shared `rate_limiter`, which is why a grep for `rate_limiter.check` missed it. Left as is.

For contrast, the endpoints that do it right: `login.rs:35`, `register_personal.rs:42`, `forgot_password.rs:25`, `reset_password.rs:27`, `report.rs:29`, `share.rs:76`.

**Fix:** add per-IP and per-token buckets to the three request endpoints, mirroring `share_pw_allowed`. Add a per-email bucket to resend-verification.

### S2-3. Shadow upload route can mix two identities. **Risk**

`routes/api/v1/shadow/file/+server.js`. `validateRequest` resolves the caller from the `X-Api-Key` header and stamps `body.owner_api_key` from it (line 74). The final create call then sends a different header:

```js
const sessionUser = await locals.session.user.get();
const apiKey = sessionUser?.api_key || request.headers.get('X-Api-Key') || undefined;
```

A logged-in browser that posts to the shadow route with someone else's `X-Api-Key` validates as that other identity but authenticates downstream as itself. It is not obviously exploitable today because `create_files` re-derives ownership, but two identities in one request is a bug waiting to become one.

**Fix:** use the single identity resolved by `validateRequest` for both the body and the header.

### S3-2. `target="_blank"` without `rel`

`routes/pricing/+page.svelte:173` and `lib/components/Footer.svelte:41`. Modern browsers imply `noopener`, but add `rel="noopener noreferrer"` so the referrer does not leak.

---

## 2. Functional bugs

### S1-7. Folder ZIP download is broken. **Confirmed**

`routes/home/files/+page.svelte:204-212`:

```js
let { data } = await FrontendClient.get('/api/v1/sanctum/file/list', { params: { folder_id: folder.id } });
if (!data?.success?.data?.files) { throw new Error('Failed to fetch folder contents'); }
const folderFiles = data.success.data.files;
```

The proxy at `routes/api/v1/sanctum/file/list/+server.js` does `return json(response.data)`, which is the raw api_switch envelope `{status, message, errors, data}`. There is no `success` wrapper. The same file reads it correctly 190 lines earlier (`fetchFilesFn`, line 17-18: `data?.status === 200` then `data?.data?.files`).

Result: "Download folder as zip" always fails with "Failed to fetch folder contents".

### S1-8. Folder delete confirmation always says "unknown". **Confirmed**

`routes/home/files/+page.svelte:376` posts to `/api/v1/sanctum/folder/stats`. That proxy route does not exist. `routes/api/v1/sanctum/folder/` contains only `create, delete, list, permanent-delete, restore, star, update`. The backend handler is there (`api_switch/src/routes/folder/folder_stats.rs`, wired at `folder/mod.rs:20`), only the SvelteKit proxy is missing. SvelteKit returns its 404 page, axios throws, and the catch sets `deletedItemCount = 'unknown'`.

So the "you are about to delete N items" safety copy never shows a number, on the one action that recursively removes a folder tree.

**Fix:** add `routes/api/v1/sanctum/folder/stats/+server.js` following the `star` proxy pattern.

**[correction]** While adding the proxy it turned out the handler also nests its payload one level deeper than every other handler, putting the numbers at `data.data.total_items`. That was flattened to match the convention. The original assumption that nothing read the endpoint was wrong: the web client could not, but the silocat testkit calls api_switch directly and pinned the nested shape in `test_foreign_folder_stats_does_not_leak`, so that test was updated alongside. The `-1` it asserts is a load-bearing no-leak sentinel (for a folder the caller does not own the recursive CTE's base case matches nothing, so `COUNT(*) - 1` is `-1`). It survives the change, and the delete dialog now treats any negative total as unknown rather than rendering "-1 items".

### S1-9. The Settings page can never show the API key. **Confirmed**

`routes/home/settings/+page.svelte:232`:

```js
let apiKey = $state(data.user?.api_key || '');
```

`routes/+layout.server.js:57-66` deliberately strips `api_key` before returning `user` to the client, with a correct and well-argued comment. `routes/home/settings/+page.server.js` returns only `{ usernameStatus }`. So `data.user.api_key` is always `undefined`, `maskedKey` is always `''`, the show/hide eye toggles between two empty strings, and "Copy" is a no-op.

The only way a user can obtain their key today is to press **Rotate**, which invalidates the key their existing integrations are using.

**Fix:** the layout policy is right. Return the key from `settings/+page.server.js` only (so it is in the payload of exactly one authenticated page), or better, add a `POST /api/v1/user/reveal-api-key` that the page calls on demand behind the eye toggle.

### S2-4. Query cache keys ignore the current folder. **Confirmed**

`routes/home/files/+page.svelte:38` and `:64` use `queryKey: ['fetchFiles']` and `['fetchFolders']` with no `currentFolderId` in the key. Navigating between folders therefore reads and writes the same cache entry, and correctness depends entirely on the manual `refreshView()` invalidation at line 193-196. The visible symptom is the previous folder's contents flashing on entry.

**Fix:** `queryKey: ['fetchFiles', currentFolderId]`. That also removes the need for most of the manual invalidation.

### S2-5. Three cache keys are invalidated but never registered, so those views go stale. **Confirmed**

| Invalidated at | Key | Actually registered as |
| --- | --- | --- |
| `files/+page.svelte:111,1385` | `fetchStarredFiles` | `starredFiles` (`ResourceList.svelte:51`) |
| `files/+page.svelte:112,1386` | `fetchStarredFolders` | `starredFolders` (`ResourceList.svelte:67`) |

No query anywhere uses `fetchStarredFiles`/`fetchStarredFolders`, so starring a file on the Files page never refreshes `/home/starred`.

The mirror problem: the dashboard registers `fetchRecentFiles` and `fetchRootFolders` (`home/+page.svelte:13,27`) and the Files page never invalidates either, so uploading or deleting leaves the dashboard showing stale recents until a hard reload. `ResourceList`'s `sharedFiles`/`sharedFolders` are likewise never invalidated by `copyShareLink`.

**Fix:** centralise the keys in one module (`$lib/queryKeys.js`) and export a `invalidateResources(queryClient)` helper that hits all of them. The current copy-paste string approach has already drifted three ways.

### S2-6. Share modal silently deletes an existing link expiry. **Confirmed**

`lib/components/ShareModal.svelte:26` initialises `expiryChoice = '0'` and never syncs it from the loaded `expiresAt` (the `onMount` handler at line 51 sets `expiresAt` but not `expiryChoice`). `applyOptions` at line 133 then always sends:

```js
payload.expires_in_days = parseInt(expiryChoice, 10) || 0;
```

Backend `share.rs:130`: `Some(_) => (true, None)` for 0, meaning "clear the expiry". So a user who opens a link that expires in 7 days, sets a password, and hits Save has just removed the expiry. The dropdown was showing "Never" the whole time, which is also simply wrong as a display of current state.

**Fix:** derive `expiryChoice` from `expiresAt` on load, and send `expires_in_days` only when the user actually changed the control. The backend already supports "leave unchanged" via `None`.

### S2-7. A file named "Folder" is treated as a folder. **Confirmed**

`lib/components/ShareModal.svelte:85` and `:157`:

```js
if (item.type === 'folder' || item.name === 'Folder') payload.folder_id = item.id;
```

Sharing or regenerating a link for any file literally named `Folder` sends `folder_id`, which matches no folder, so the request 404s. The `item.name` clause looks like a workaround for callers that omit `type`.

**Fix:** always pass `type` from the call sites and drop the name check.

### S2-8. Settings theme control desyncs from the sidebar toggle. **Confirmed**

`routes/home/settings/+page.svelte:10` imports `theme as themeStore` and then never uses it. Line 38 declares a separate local `let theme = $state('dark')`, seeded once in `onMount` from `localStorage` (line 195). The sidebar's `ThemeToggle` writes through `$lib/theme.js`, which updates the store and `localStorage` but not this local copy. Toggle the theme from the rail while Settings is open and the Settings radio buttons keep showing the old value.

**Fix:** delete the local `theme` state and bind to `$themeStore`.

### S2-9. Two dead server-side loads on every navigation. **Confirmed**

`routes/home/+page.server.js` and `routes/home/files/+page.server.js` both do a full `fetch('/api/v1/sanctum/file/list')` and return `{ files }`. Neither page component reads `data.files`: the dashboard has no reference to it, and the Files page shadows `data` inside every query function and uses TanStack Query instead.

Every navigation to those two routes therefore pays for one extra full file listing round trip through the proxy to api_switch, blocking first paint, for a result nobody reads.

**Fix:** delete both `+page.server.js` files, or use them and drop the client query.

### S3-3. Pluralisation and edge cases in `formatTime` / `formatSize`

`routes/home/files/+page.svelte:401-419`. `formatTime` (line 410) renders "2 hour ago" and "1 days ago". `formatSize` has no clamp on the `sizes` index, so `formatSize(0.5)` indexes `sizes[-1]` and returns `undefined`.

The dashboard (`home/+page.svelte:113`) and the sidebar (`Aside.svelte:54`) each carry their own corrected copy with `Math.min(..., sizes.length - 1)`. There are now at least three divergent copies of `formatSize` and two of the relative-time helper.

**Fix:** one `$lib/format.js` with `formatSize` and `relativeTime`, used everywhere.

### S3-4. `getFileType(mime)` throws on a null mime

`routes/home/files/+page.svelte:422`: `if (mime.includes('image'))`. Any file row whose `mime` is null crashes the render of that card. `glyphForMime` in `lib/ui/icons.js` handles this properly and takes a filename fallback; use it here too.

---

## 3. UX and performance

### S1-10. Downloads assemble the entire file in RAM, so large files cannot be downloaded at all. **Confirmed**

All three download paths do the same thing:

- `lib/download.js:76-91` (`downloadFile`) collects every decrypted chunk into `parts[]` and then `new Blob(parts)`.
- `lib/download.js:140-155` (`fetchDecryptedBlob`, used for inline preview).
- `routes/s/[token]/+page.svelte:160-190` (the public share page).

`CHUNK_SIZE` is 100 MB and the upload route accepts up to 20 GB anonymous / 50 GB authenticated (`routes/api/v1/shadow/file/+server.js:59-60`). A 5 GB file needs roughly 10 GB of resident memory during assembly (the chunk array plus the Blob copy). Desktop Chrome dies somewhere around 1-2 GB; mobile Safari dies far below that.

This is the single biggest gap between what the product sells and what it does. You can upload a 20 GB file and then never get it back.

**Fix:** stream to disk. `showSaveFilePicker()` -> `FileSystemWritableFileStream` covers Chromium desktop and lets you write each decrypted chunk and release it. Fall back to a service-worker-mediated `ReadableStream` response (the StreamSaver pattern) elsewhere. Both keep peak memory at one chunk. Dropping `CHUNK_SIZE` to 8-16 MB would also make progress smoother and retries cheaper.

`fetchDecryptedBlob` for preview has the same problem with an extra twist: previewing a 4 GB video downloads all 4 GB before showing anything. Preview should fetch chunk 0 only, or be disabled above a size threshold.

### S1-11. Argon2id and chunk decryption run on the main thread during download. **Confirmed**

The upload path solved this properly: `routes/home/files/+page.svelte:775-800` posts to `$lib/workers/crypto.worker.js`, and the comment at line 794 explains exactly why. The download path did not. `lib/download.js:6` imports `decryptChunk, deriveKeyFromPassword` straight from `$lib/chacha.js`, which runs on the main thread.

`crypto_pwhash` at `OPSLIMIT_MODERATE` / `MEMLIMIT_MODERATE` is 256 MiB and roughly a second of blocking compute. Decrypting a 100 MB chunk blocks again. The tab is completely frozen for the duration: no spinner animation, no cancel button, no scrolling. The "Deriving key…" phase label at `download.js:69` is written but cannot paint, because the very next statement blocks the thread that would paint it.

`lib/requestUpload.js:10` has the identical problem on the inbound file-request page.

**Fix:** route all three through the existing worker. The infrastructure is already built and tested.

### S1-12. The file-request page loads the whole file into memory to hash it. **Confirmed**

`lib/requestUpload.js:19-22`:

```js
async function sha256Hex(file) {
    const digest = await crypto.subtle.digest('SHA-256', await file.arrayBuffer());
    ...
}
```

`file.arrayBuffer()` materialises the entire file. The main upload path already solved this with the worker's chunked `hashFile` (`files/+page.svelte:820-821`). Sending anything over about 1 GB through a file-request link fails with an out-of-memory error before a single byte is uploaded.

**Fix:** reuse `callWorker('hashFile', ...)`.

### S2-10. Nothing warns before a navigation kills an in-flight transfer. **Confirmed**

`grep -rn "beforeunload"` across `routes` and `lib` returns nothing. For a product whose core action is a multi-gigabyte transfer, closing the tab, hitting reload, or clicking a nav link silently destroys hours of upload with no prompt.

**Fix:** register a `beforeunload` handler while `isUploading` or any `downloads` store entry is `active`. Add SvelteKit's `beforeNavigate` guard for in-app navigation, which is the more likely accident.

### S2-11. `height: 100vh` cuts off content on mobile. **Confirmed**

`routes/home/+layout.svelte:66` sets `.layout { height: 100vh; overflow: hidden }` and `lib/components/Aside.svelte:194` sets `height: 100vh`. On iOS Safari and Android Chrome, `100vh` is the viewport height *without* the URL bar, so the layout is taller than the visible area. Combined with `overflow: hidden` on the parent, the bottom of the sidebar (which holds the storage meter and the account row) and the bottom of the content pane are unreachable.

**Fix:** `height: 100dvh` with a `100vh` fallback. Same for `ShareModal.svelte:349`.

### S2-12. Two hand-rolled modals bypass the accessible one. **Confirmed**

`lib/ui/Modal.svelte` is well built: Escape to close (line 25), `role="dialog"` and `aria-modal` (line 45), body scroll lock (line 28-33), a `dismissible` flag for mid-operation. It is used by exactly three components (`MoveToModal`, `InputModal`, `Navbar`).

`ShareModal.svelte:187` and `routes/home/files/+page.svelte:2241` and `:2377` hand-roll `.modal-backdrop` divs instead. Those dialogs have:

- no Escape-to-close
- no `role="dialog"` / `aria-modal` (svelte-check flags both: `a11y_no_static_element_interactions`, `a11y_click_events_have_key_events`)
- no body scroll lock, so the page scrolls behind the dialog on wheel and touch
- no focus management

The upload modal in particular is the app's primary action surface.

**Fix:** port all three onto `$lib/ui/Modal.svelte`. While there, add a focus trap and focus restore to `Modal.svelte` itself, which it currently lacks.

### S2-13. Mobile drawer has no Escape and no focus trap

`routes/home/+layout.svelte:32` renders the scrim as a bare `<div role="presentation" onclick>`. Escape does nothing, and tab order still walks the page behind the drawer.

### S2-14. Folder navigation is invisible to the URL. **Confirmed**

`routes/home/files/+page.svelte:162-175`. `currentFolderId` and `folderPath` are component state only. Consequences:

- browser Back does not go up a folder, it leaves the Files page entirely
- reloading inside a folder drops the user at root
- a folder cannot be bookmarked, linked, or shared with a teammate
- opening the same folder in a new tab is impossible

**Fix:** put the folder in the URL (`/home/files?folder=<id>`), drive `currentFolderId` from `$page.url.searchParams`, and reconstruct the breadcrumb from a parent chain the folder list already returns. This also fixes S2-4 for free, since the query key can then key off the URL.

### S2-15. "Encrypted in your browser" is shown while encryption is off. **Confirmed**

`routes/home/files/+page.svelte:2433` renders the drop zone sub-text "Any file type, encrypted in your browser", but the encryption toggle immediately below it (line 2438) defaults to **off**. Files dropped in the default state are uploaded in plaintext and marked `public_access: true` (S1-2).

For a product positioned on zero-knowledge this is the most damaging copy in the app: it tells the user encryption is happening at the exact moment it is not.

**Fix:** make the sub-text reflect the toggle state, and consider defaulting the toggle on. If the default stays off, the drop zone should say so plainly.

### S2-16. Regenerating a share link is destructive with no confirmation

`ShareModal.svelte:155-173`. One click permanently invalidates the link the user has already sent to people, with no confirm step and no "are you sure, N people have opened this" context, even though the access-log data needed for that context is already loaded into `accessEvents`.

### S2-17. One-time links burn on authorize, not on successful download

`api_switch/src/routes/file/share.rs:794-800` increments `link_downloads` inside `public_authorize_download`, before the client has fetched a single chunk. A network failure, a closed tab, or a browser that cancels the request consumes the recipient's only attempt and they get "This safe-link has expired" with no recourse.

**Fix:** either confirm consumption from the client after the final chunk lands, or give "once" links a small grace count and surface the remaining attempts in the UI.

### S3-5. The public share page is the only component still in legacy mode. **Confirmed**

`routes/s/[token]/+page.svelte` contains no runes at all (`$state`, `$derived`, `$props`, `$effect` all absent), so `let loading = true` / `let file = null` are reactive only because Svelte 5 falls back to legacy mode for rune-free components. The neighbouring `routes/req/[token]/+page.svelte` is fully runed.

This is not broken today, but adding a single rune anywhere in that file flips it to runes mode and **every one of those `let` bindings silently stops updating**. The failure would land on the highest-traffic unauthenticated page in the product, where a recipient sees a permanent "Loading…".

**Fix:** migrate it to runes now, while the file is small.

### S3-6. Misleading 410 copy on the share page

`routes/s/[token]/+page.svelte:49-50` maps every 410 to `"This 'Once' link has expired."`, but the backend returns 410 for date-expiry too (`share.rs:786`). A user whose link hit its expiry date is told it was a one-time link.

The backend already sends distinct messages. Surface `e.response.data.message` instead of hardcoding.

### S3-7. Encrypted-file download progress can exceed 100%

`routes/s/[token]/+page.svelte:180`: `completedBytes += chunk.size` where `chunk.size` is the ciphertext length, divided by `fileMeta.size` which is the plaintext length. Each chunk adds a 16-byte Poly1305 tag, so the bar overshoots. `lib/download.js:60` gets this right by summing the chunk sizes for the total.

### S3-8. The file-request page is noticeably thinner than the rest of the app

`routes/req/[token]/+page.svelte` is the first thing an outside recipient ever sees of Silocat, and compared to every other surface it is missing:

- no drag and drop (the main app has it)
- single file only
- no size display, no quota hint, no size-limit error until the upload fails
- no "Generate" button next to the encryption password, though `$lib/password.js` exists and the main app offers it
- no cancel during upload
- no guidance on how to get the password to the recipient, which is the one thing the sender must do out of band
- footer says "files are stored encrypted at rest", which is true of R2 but reads as an E2E claim when the encryption checkbox is unchecked

This page converts strangers into users. It deserves the same treatment as the landing page.

### S3-9. No `prefers-color-scheme` respect

`app.html:9` and `lib/theme.js:9` both default to `'dark'` when `localStorage` has nothing. A first-time visitor on a light-mode OS gets forced dark. Use `window.matchMedia('(prefers-color-scheme: light)')` as the fallback instead of a hardcoded `'dark'`.

### S3-10. Fragile column-hiding selectors

`routes/home/files/+page.svelte:3931-3935` hides list columns on narrow screens with `.lhead span:nth-child(3)` and `.lrow > .lmeta:nth-of-type(2)`. Reordering or inserting a column silently hides the wrong one. Use a class per column.

### S3-11. Folder ZIP requires "decrypt a file first"

`routes/home/files/+page.svelte:228-239` gates encrypted folder ZIP on the module-level `decryptionPassword`, with an error message that tells the user to "decrypt a file first to unlock session keys". The comment above it is a stream of unresolved questions from the author. Per-file passwords are the model everywhere else in the app; the bulk-password modal at line 2239 already solves exactly this. Route the folder ZIP through it. (Moot until S1-7 is fixed, since the path never gets this far.)

### S3-12. `error` on a 404 inside `/home` sends the user to the marketing site

`routes/+error.svelte:27-30` offers "Back to home" (`/`) and "See pricing". A signed-in user who hits a bad `/home` URL should be offered `/home`, not a pricing page.

---

## 4. Housekeeping

### S3-1. Debug logging left in production paths

23 `console.log` calls across `routes` and `lib`. Beyond the credential leaks already listed under S1-5 and S1-6:

```
routes/api/v1/shadow/file/+server.js:26    console.log("[log 1]");
routes/api/v1/shadow/file/+server.js:45    console.log({ error });
routes/api/v1/sanctum/file/star/+server.js:13,21,22
routes/api/v1/sanctum/folder/star/+server.js:13,20,21
routes/home/files/+page.svelte:45,46,50,316,321
routes/auth/signin/+page.server.js:53
```

Note `shadow/file/+server.js:45-46` and `:26`: `[log 1]` fires on every anonymous upload.

**Fix:** delete, or gate behind `import { dev } from '$app/environment'`.

### S3-13. Dead code

- `lib/frontendClient.js:4-8`: `PublicApiRoutes` is declared and never exported or used.
- `lib/network.js:31-64`: `ApiServerRoutes` lists `earlyAccess`, `startChunkUpload`, `stopChunkUpload`, `startChunkDownload`, `startChunkDelete`, `getFolder`, `downloadFile`, `markChunkAsUploading` plus a commented-out `createFile`. Several no longer have backends.
- `ShareModal.svelte:46`: `?user_id=${window.currentUser?.id || ''}` where `window.currentUser` is set nowhere in the codebase, so it always sends an empty parameter. The two comment lines below it are the author asking themselves whether the proxy needs it. (It does not: `share/info/[id]/+server.js:14` supplies `user.id` server-side.)
- `routes/home/settings/+page.svelte:10`: `themeStore` imported, never used (see S2-8).
- `routes/home/settings/+page.svelte:664`: unused CSS selector `.f-field textarea.mono`.
- `ShareModal.svelte:343`: six unused `.opts .receipts*` selectors, suggesting a receipts UI that was moved but whose styles were left behind.

### S3-14. svelte-check baseline

`npx svelte-check` reports 894 errors and 27 warnings across 128 files. Almost all are `implicitly has an 'any' type` from `checkJs` running over untyped JS, so the signal is buried. Either turn `checkJs` off in `jsconfig.json` and keep svelte-check for a11y and Svelte diagnostics only, or commit to JSDoc types. Right now the check is running in CI shape but nobody can read it.

Real warnings worth acting on from that run:

- `routes/home/settings/+page.svelte` 136, 187-190, 232, 265: six `state_referenced_locally`, the Svelte 5 form of "this captured the initial value and will not update".
- `lib/components/ContextMenu.svelte:9`: same, for the menu's `x`/`y` position.
- `lib/components/FolderCard.svelte:78`: `a11y-click-events-have-key-events` written in the Svelte 4 dash form, so the suppression comment does nothing.

### S3-15. `robots.txt` coverage

`static/robots.txt` disallows `/home`, `/auth`, `/s/`, `/api/v1/` but not `/req/` (inbound file-request pages) or the anonymous `/[slug]` drop pages. Both carry `<meta name="robots" content="noindex">` so they are covered in practice, but the two lists should agree.

### S3-16. Toast import inconsistency

`ShareModal.svelte:4` imports `toast` from `svelte-sonner` directly; every other component uses `$lib/toast.js`, whose wrapper takes `(title, description)` as two strings. Sonner's native signature is `(title, options)`. ShareModal only ever passes one argument so nothing breaks today, but the next person to add a description string there will pass it where an options object is expected.

---

## What shipped

Branch `fix/audit-2026-08-23`, eight commits, nothing deployed.

| Commit | Covers |
| --- | --- |
| `docs: audit findings` | this document |
| `security: CSPRNG for share passwords, stop logging credentials` | S1-1, S1-5, S1-6, S3-1 |
| `security: files in a registered drive are private until explicitly shared` | S1-2, S1-3, S1-4, plus migration 0044 |
| `fix: repair three features that could not work at all` | S1-7, S1-8, S1-9, S2-8, S2-9 |
| `perf: stream downloads to disk, move all crypto off the main thread` | S1-10, S1-11, S1-12, S2-10, S3-5, S3-6, S3-7 |
| `fix: folder navigation in the URL, one source of truth for cache keys, modal a11y` | S2-4, S2-5, S2-6, S2-7, S2-11, S2-12, S2-13, S2-14, S2-15, S2-16, S3-8, S3-11 |
| `security: ship a real CSP, meter the unauthenticated endpoints` | S2-1, S2-2, S2-3, S3-9 |
| `polish: shared formatters, a11y fixes, readable svelte-check` | S2-17 (partial), S3-2, S3-3, S3-4, S3-10, S3-12, S3-13, S3-14, S3-15, S3-16 |

### Things found while fixing, not in the original list

* **The Files context menu's "Copy link" was broken by design.** It built `${origin}/${item.id}`, the anonymous-drop route, which resolves by raw file id through the `public_access` gate. It only ever produced a working URL because of the default S1-2 removes, and it never turned sharing on. Now goes through the token flow.
* **Delivery-receipt styles never applied.** The `.receipts*` rules were nested under `.opts` in the SCSS while the markup renders them as its sibling, so that list has been unstyled since it shipped. Hoisted.
* **`folder_stats` double-nested its payload** (see the S1-8 correction).
* **A duplicate `send` key** in the icon map, flagged by svelte-check under the noise.
* **`FolderCard` was keyboard-focusable but not keyboard-activatable**, and its suppression comment used the Svelte 4 dash spelling so it suppressed nothing.

### Deliberately left alone

* **S2-17, one-time links still burn at authorization.** Authorization happens before any bytes move, so a dropped connection still costs the recipient their attempt. The obvious fix, spending the link on a post-download confirmation, lets a client simply never confirm and download forever, which is worse than the bug. The counter stays put; the page now warns the recipient before they start, and a spent link says it was already used rather than "safe-link has expired". A proper fix needs a resumable-session protocol and is its own piece of work.
* **Ten `state_referenced_locally` warnings.** All seed-from-prop patterns where only the initial value is wanted (`Input`'s generated uid, `InputModal`'s `initialValue`, `ResourceList`'s `variant`) or where an explicit `$effect` re-syncs (`settings`' `profileForm`).
* **`checkJs`.** Turned off rather than satisfied. Annotating ~130 untyped files was out of scope for one pass; the config comment says how to reintroduce it file by file.

### Before this goes out

* **The CSP is the one change that can break the site silently, and it has never run in a browser.** Load staging with devtools open and watch for violations before promoting. The likely candidates are an origin I did not find (an analytics or payment host) and the pinned hash of the inline theme script: edit `src/app.html` without regenerating it and every visitor falls back to dark. The regeneration command is in `_headers`.
* **Streaming downloads have never run in a browser either.** Three tiers, and only the Blob one is the old code path. Test a large file in Chromium (File System Access), Firefox (service worker) and Safari, encrypted and not.
* **Migration 0044 has run on dev only.** It flipped 14 sanctum files private and cleared 2 stale tokens there. On production it will touch more, and it deliberately leaves live shares alone.
* **`.svelte-kit/` in this working tree is root-owned** from an old container build, so `vite build` needs `SVELTEKIT_OUT_DIR=.svelte-kit-local`. Worth a `sudo chown` at some point; it is not something this branch changed.
