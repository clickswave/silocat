# @silocat/client

Client for the [Silocat](https://silo.cat) API. It does the end-to-end
encryption for you, so ciphertext is all that ever leaves your process.

```bash
npm install @silocat/client
```

## Why this exists

Silocat is zero-knowledge. Files are encrypted before upload with a key derived
from a password the server never sees, and the raw HTTP API reflects that: it
takes ciphertext and hands back presigned URLs. Talking to it directly means
implementing Argon2id key derivation, chunking, and XChaCha20-Poly1305 with a
fresh nonce per chunk, exactly as the web app does.

Getting any of that subtly wrong produces files that upload cleanly and then
cannot be decrypted by anyone, including you. This library is the reference
implementation so you do not have to take that risk.

## Quick start

Get an API key from **Settings → API** in the web app.

```js
import { Silocat } from '@silocat/client';

const silo = new Silocat({ apiKey: process.env.SILOCAT_API_KEY });

// Upload. The plaintext never leaves this process.
const file = await silo.upload(bytes, {
  name: 'contract.pdf',
  mime: 'application/pdf',
  password: 'correct-horse-battery-staple',
  onProgress: ({ ratio }) => console.log(`${Math.round(ratio * 100)}%`)
});

// Share it. The link carries no key.
const { url } = await silo.share(file.id, { type: 'public' });

// Read it back.
const plaintext = await silo.download(file.id, {
  password: 'correct-horse-battery-staple'
});
```

Self-hosting? Point it at your instance:

```js
new Silocat({ apiKey, baseUrl: 'https://files.example.com' });
```

## The two things to understand

**1. Lose the password, lose the file.** There is no recovery, no reset, no
support ticket that gets it back. That is the product working correctly. Store
passwords in whatever you already use for secrets.

**2. A share link is not a key.** `share()` returns a URL anyone can open, but
an encrypted file still needs its password, which the link never carries. Send
it through a different channel. A leaked link on its own decrypts nothing, and
that property only holds if you keep them separate.

## API

| Method | Notes |
|---|---|
| `storage()` | `{ used, total, free }` in bytes |
| `listFiles({ folderId, starred, shared })` | omit `folderId` for the root |
| `listFolders({ parentId })` | |
| `createFolder(name, { parentId })` | |
| `upload(data, { name, password, mime, folderId, onProgress })` | `Uint8Array`, `ArrayBuffer` or `Blob`. Omit `password` to upload unencrypted, readable by the server. |
| `download(id, { password, onProgress })` | returns `Uint8Array` |
| `share(id, { type })` | `'public'`, `'once'` or `'off'` |
| `unshare(id)` | |
| `trash(id)` | recoverable until the retention window expires |
| `restore(id)` | |
| `deleteForever(id)` | irreversible |

Errors are `SilocatError` with `.status` and `.body`.

## Requirements

Node 18+ (needs global `fetch` and WebCrypto). In older runtimes pass your own:

```js
new Silocat({ apiKey, fetch: myFetch });
```

Argon2id at libsodium's MODERATE limits is deliberately expensive, roughly a
quarter second and 256 MB per derivation. That cost is the point: it is what
stands between a leaked ciphertext blob and its plaintext. Derive once and reuse
the client if you are moving many files under one password.

## Licence

AGPL-3.0-only, same as Silocat itself.
