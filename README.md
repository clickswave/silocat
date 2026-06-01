# silocat

End-to-end-encrypted file sharing and object storage. Upload, organize, and share files behind safe links. Lives at [silo.cat](https://silo.cat).

## Overview

silocat is a file-sharing / object-storage product. Users upload files (chunked
and client-side encrypted) into folders, organize them, star them, soft-delete
and restore them, and share any file or folder via a tokenized public link.

Core features:

- Chunked uploads with resume support; large files are split into chunks that are
  uploaded directly to object storage via pre-signed URLs.
- Client-side encryption. The browser encrypts chunks with XChaCha20-Poly1305
  (libsodium); password-derived keys use Argon2/`crypto_pwhash`. The server only
  ever stores ciphertext plus per-chunk `nonce`/`salt` and a SHA-256 checksum.
- Folders, starring, trash (soft delete) and restore, permanent delete.
- Shareable links per file or folder. Each gets a random `share_token`; the
  `share_type` is `off`, `public`, or `once`. `once` ("safe-link") enforces a
  download limit (`link_max_downloads`) and expires (HTTP 410) once reached.
  Tokens can be regenerated, which resets the download counter.
- Anonymous ("shadow") uploads as well as authenticated ("sanctum") storage
  (see Architecture).
- Accounts via email/password or Google OAuth, with email verification and
  Cloudflare Turnstile on auth flows.
- Razorpay-based subscriptions / billing (orders, verification, promo codes,
  usage tracking) and an admin dashboard.

## Architecture

Three services plus an nginx proxy, all on the shared `clickswave_network`:

- **web_server** — SvelteKit frontend (Svelte 5), the silo.cat user app. Deploys
  to Cloudflare Pages (`@sveltejs/adapter-cloudflare`). Handles client-side
  encryption (libsodium), uploads via pre-signed URLs, zipping downloads (jszip),
  Turnstile, and Razorpay checkout. Dev port 12001.
- **api_switch** — Rust / Axum backend (the "API switch"). Listens on `0.0.0.0:31337`.
  Owns all data and storage logic: user/auth, files/folders, chunks, sharing,
  billing, and admin routes. Runs its own `sqlx` migrations on startup and gates
  startup on required env vars. Every request is authenticated by an
  `X-Authority-Sign` header (`AUTHORITY_SIGN`) shared with the frontends; public
  share endpoints (`/file/public/share/*`) take no user token.
- **admin_dashboard** — SvelteKit admin app (charts via chart.js) for users,
  files, orders, subscriptions, promos, invites, early-access, and stats. Dev
  port 12002.
- **nginx** — reverse proxy in front of the API (host port 8080 in dev).

Object storage is **Cloudflare R2** (S3-compatible, via the AWS Rust SDK in
`services/api_switch/src/libs/r2.rs`). There are two buckets / R2 clients:

- **sanctum** (`silo-cat-sanctum`) — storage for authenticated users (files with
  a `user_id`).
- **shadow** (`silo-cat-shadow`) — storage for anonymous / unauthenticated
  ("shadow") uploads.

The backend picks the bucket per file based on whether it has an owner, and
generates pre-signed PUT/GET URLs (24h expiry) so clients upload/download chunks
directly to R2. A CDN base URL (`PUBLIC_CDN_BASE_URL`) with a signing secret
(`CDN_SECRET`) fronts public delivery.

Postgres is **core's shared `silocat` database** — `api_switch` connects via
`DATABASE_URL` pointing at core's `core_postgres`, and runs its own migrations
(`services/api_switch/migrations/`) against it. The `services/postgres` dir is a
local convenience only.

## Running locally

silocat is orchestrated by the monorepo's `manager.sh`, which composes the
per-service `orchestration/docker-compose.dev.yml`. It depends on **core**
(which provides Postgres), so bring up core first.

```bash
# from the monorepo root
./manager.sh local-dev --start --only core,silocat
./manager.sh local-dev --status --only silocat
./manager.sh local-dev --logs   --only silocat
./manager.sh local-dev --stop   --only silocat
```

Dev endpoints: web_server `:12001`, admin_dashboard `:12002`, api_switch
`:31337`, nginx `:8080`. The `clickswave_network` Docker network must exist
(created by core).

## Configuration

Config is supplied per environment via `env/silocat.env.<env>` (dev / staging /
prod) at the monorepo root; these files are gitignored and hold secrets. Key
groups:

- **Database**: `DATABASE_URL`, `POSTGRES_DATABASE_URL` (point at core's
  `silocat` DB).
- **Auth / internal**: `AUTHORITY_SIGN`, `JWT_SECRET`, `INTERNAL_API_URL`.
- **Google OAuth**: `OAUTH_ID_GOOGLE`, `OAUTH_SECRET_GOOGLE`,
  `OAUTH_CALLBACK_BASE_URI`.
- **Cloudflare R2**: `CF_R2_SHADOW_API_URL`, `CF_R2_SANCTUM_API_URL`,
  `CF_R2_ACCESS_ID`, `CF_R2_ACCESS_SECRET`, `CF_R2_REGION`, `CF_R2_BUCKET`
  (the backend also reads these as `R2_*_ENDPOINT` / `AWS_*`).
- **CDN**: `PUBLIC_CDN_BASE_URL`, `CDN_SECRET`.
- **Billing**: `RAZORPAY_ID`, `RAZORPAY_SECRET`.
- **Email (Postmark over SMTP)**: `SMTP_ADDRESS`, `SMTP_USERNAME`,
  `SMTP_PASSWORD`.
- **Turnstile**: `PUBLIC_TURNSTILE_KEY`, `TURNSTILE_SECRET`.

Never commit secrets. (The folder also contains stray local artifacts such as a
`vps-key.pem` and `*.txt` notes that are not part of the app.)

## Part of the clickswave monorepo

silocat is one project in the clickswave monorepo. It shares the orchestration
tooling (`manager.sh`), the `clickswave_network`, and the `core` service's
Postgres. Domains are mapped centrally (`silo.cat` for prod, `stage.silo.cat` for
staging).
