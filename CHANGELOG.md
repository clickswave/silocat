# Changelog

Notable changes to Silocat. This project keeps a human-readable changelog; dates
are ISO-8601.

## [Unreleased]

### Security
- Email-change verification is rate-limited and compared in constant time (it was
  a brute-forceable 6-digit code with no lockout).
- The account API key is no longer serialized into page hydration, so client-side
  JavaScript can never read it.
- Deleting a file now revokes its public share link immediately.
- The by-id file endpoints honor share protections (password, expiry, one-time),
  so a protected share can no longer be bypassed by guessing the file id.
- "Permanently delete" now removes the encrypted objects from storage, for a
  single file and for whole folder subtrees.
- The email-change request is throttled (it could be used to mail-bomb an
  arbitrary address), and the forgot-password flow no longer reveals whether an
  account exists.
- Storage stats require authentication and report only the caller's own usage
  (previously any user's stats could be read by id).
- The session token pins its signing algorithm, and the public-share proxies
  forward the real client IP so per-IP throttles work.
- Storage quota is measured from the real stored object size, not a
  client-declared value.

### Added
- Request a file: create a link and let anyone send files straight into your
  account, with no account needed on their end.
- Delivery receipts: see who opened a shared link, and when.
- Recurring subscriptions through Razorpay. Until plan ids are configured, plan
  purchases fall back to one-time orders, so nothing breaks.
- Production self-hosting via a standalone Node server (`@sveltejs/adapter-node`),
  built by `Dockerfile.selfhost`.

### Changed
- Encryption is described honestly as optional (on when you turn on password
  protection) across the app, marketing, and docs.
- Shared downloads no longer send the decryption password to the server; it stays
  in the browser and is used only to decrypt locally.
- Pricing: every privacy feature stays free on every tier. Paid plans add space
  and delivery workflow tools (request-a-file and delivery receipts), not privacy.

### Fixed
- Subscription renewals and upgrades no longer stack the storage quota.
- The billing webhook re-verifies the captured amount and currency before it
  grants a plan.
- Plan grants come from one canonical table, fixing an invited-Pro grant of 1 TiB
  while paid Pro grants 2 TB.

### Removed
- The unused early-access route and the `validator` dependency, along with some
  dead code and debug logging.
