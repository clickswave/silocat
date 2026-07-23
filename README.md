# Silocat

Zero-knowledge, end-to-end encrypted file sharing and storage. Drop a file, share a link, nobody in between can read it. Live at [silo.cat](https://silo.cat).

Silocat is open source under [AGPL-3.0](LICENSE). Run it yourself, read the crypto, or use the hosted version.

## What it does

- **Client-side encryption.** The browser encrypts chunks with XChaCha20-Poly1305 (libsodium); password-derived keys use Argon2 (`crypto_pwhash`). The server only ever stores ciphertext plus per-chunk `nonce`/`salt` and a SHA-256 checksum. Lose the password and the file is gone: no recovery, no backdoors.
- **Anonymous drops.** No account needed: a local browser key manages your uploads across sessions. Share the link before the upload even finishes.
- **Chunked, parallel transfer.** Large files are split into chunks uploaded directly to object storage via pre-signed URLs, and downloads pull chunks in parallel.
- **Folders, starring, trash, restore.** Whole directory trees upload with structure intact.
- **Share links** per file or folder: `off`, `public`, or `once` (self-destructs after a download limit), with optional link passwords. Tokens can be regenerated.
- **Accounts** via email/password or Google OAuth, with email verification and Cloudflare Turnstile.

## Self-hosting

Everything you need is in this repo, including a bundled Postgres and MinIO (S3-compatible storage), so a laptop or a single VPS is enough:

```bash
git clone https://github.com/clickswave/silocat
cd silocat
cp env.selfhost.example .env    # edit: set the passwords and secrets
docker compose -f orchestration/docker-compose.selfhost.yml --env-file .env up -d
```

The app comes up at `http://localhost:12001`. Optional integrations (SMTP email, Google OAuth, Turnstile, Razorpay billing, GeoIP) activate when their env vars are filled in; all are off by default. See `env.selfhost.example` for the full reference.

Notes for self-hosters:

- **Storage**: defaults to the bundled MinIO. Point the `CF_R2_*` variables at Cloudflare R2, Backblaze B2, or any S3-compatible store for production.
- **GeoLite2**: GeoIP lookups need MaxMind's free `GeoLite2-City.mmdb`, which their license does not allow us to redistribute. Download it yourself and set `GEOLITE2_DB_PATH`, or leave it blank to disable GeoIP.
- **Web deployment**: the hosted silo.cat serves the frontend from Cloudflare Pages (`@sveltejs/adapter-cloudflare`). The self-host compose runs the dev server for simplicity; for a hardened deployment either put it behind a reverse proxy or switch to `@sveltejs/adapter-node`.

## Architecture

Two services, one scheduler:

- **web_server**: SvelteKit (Svelte 5) frontend. Client-side encryption (libsodium), uploads via pre-signed URLs, zip downloads (jszip), Turnstile, Razorpay checkout. Dev port `12001`.
- **api_switch**: Rust / Axum backend on `0.0.0.0:31337`. Owns all data and storage logic: auth, files/folders, chunks, sharing, billing, admin routes. Runs its own `sqlx` migrations on startup. Requests between the frontend and the API carry a shared `X-Authority-Sign` header (`AUTHORITY_SIGN`); public share endpoints take no user token.
- **WatchCat**: the same `api_switch` binary started with `WATCHCAT_MODE=1`. Runs scheduled cleanup: expiring anonymous drops, orphaned-upload garbage collection.

Storage is any S3-compatible object store, addressed as three buckets (`shadow` for anonymous uploads, `sanctum` for account storage, `dp` for display pictures). The backend generates pre-signed PUT/GET URLs so clients transfer chunks directly against storage; ciphertext is all the store ever sees.

The full REST API is documented in [`openapi.yaml`](openapi.yaml).

## Development

```bash
docker compose -f orchestration/docker-compose.dev.yml up
```

The dev compose expects the monorepo layout used by the hosted deployment (external `clickswave_network`, shared Postgres, `env/silocat.env.dev`). For hacking on Silocat standalone, use the self-host compose above instead: it is fully self-contained.

## License

[AGPL-3.0](LICENSE). If you run a modified Silocat as a network service, the AGPL requires you to offer your users the modified source. That is deliberate: the trust story of a zero-knowledge product depends on the code staying inspectable, wherever it runs.

Built by [Clickswave Labs](https://clickswave.org).
