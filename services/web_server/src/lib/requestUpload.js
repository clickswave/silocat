// Client-side upload flow for "request a file" (the inbound delivery page).
//
// Self-contained so it never touches the anonymous-drop upload code: it chunks a
// File, optionally encrypts each chunk end to end (XChaCha20-Poly1305 with an
// Argon2id key, exactly like a normal encrypted upload), PUTs each chunk to its
// presigned URL, and marks it complete. All three request endpoints are public
// (authorized by the request token), so no account or api key is involved.

import axios from 'axios';
import { generateSalt, generateNonce } from '$lib/chacha.js';
import { hashFile, deriveKey, encryptChunk } from '$lib/cryptoClient.js';
import { CHUNK_SIZE } from '$lib/chunking.js';


function b64(bytes) {
	return btoa(String.fromCharCode(...bytes));
}

/**
 * Upload one File to a request.
 * @param {string} token         the request token
 * @param {File}   file          the file to send
 * @param {object} opts
 * @param {boolean} opts.encrypt   encrypt end to end (default false)
 * @param {string}  opts.password  password used to derive the key (required if encrypt)
 * @param {string}  opts.uploaderName  optional name shown to the owner
 * @param {(pct:number)=>void} opts.onProgress  0..100 progress callback
 * @returns the created file record
 */
export async function uploadToRequest(token, file, opts = {}) {
	const { encrypt = false, password = '', uploaderName = '', onProgress } = opts;

	// 1. Plaintext checksum + (optional) key derivation.
	//
	// Both go through the worker. This used to call
	// crypto.subtle.digest(await file.arrayBuffer()), which materialises the whole
	// file in memory before a single byte is uploaded, so sending anything much
	// over a gigabyte through a request link died with an allocation failure. The
	// main upload path already hashed in slices; this one did not. Argon2id was
	// likewise on the main thread here, freezing the page for the recipient of the
	// link, who is usually not even a user yet.
	const fileChecksum = await hashFile(file, CHUNK_SIZE);
	let key = null;
	let salt = null;
	if (encrypt) {
		if (!password) throw new Error('A password is required to encrypt.');
		salt = generateSalt();
		key = await deriveKey(password, salt);
	}

	// 2. Chunk metadata (per-chunk nonce; shared salt).
	const totalChunks = Math.max(1, Math.ceil(file.size / CHUNK_SIZE));
	const chunksMeta = [];
	for (let i = 0; i < totalChunks; i++) {
		const start = i * CHUNK_SIZE;
		const end = Math.min(start + CHUNK_SIZE, file.size);
		const nonce = encrypt ? generateNonce() : null;
		chunksMeta.push({
			start,
			end,
			size: end - start,
			checksum: 'pending',
			nonce: nonce ? b64(nonce) : null,
			salt: encrypt ? b64(salt) : null,
			_rawNonce: nonce
		});
	}

	// 3. Create the upload (server hands back presigned PUT urls).
	const payload = {
		token,
		uploader_name: uploaderName || null,
		file_encrypted: encrypt,
		file_name: file.name,
		file_mime: file.type || 'application/octet-stream',
		file_size: file.size,
		chunks: chunksMeta.map((c) => ({
			start: c.start,
			end: c.end,
			size: c.size,
			checksum: c.checksum,
			salt: c.salt,
			nonce: c.nonce
		})),
		sha256_checksum: fileChecksum,
		blake3_checksum: ''
	};
	const res = await axios.post('/api/v1/public/request/upload', payload);
	const data = res.data?.success?.data ?? res.data?.data;
	if (!data || !Array.isArray(data.chunks)) {
		throw new Error(res.data?.message || 'The request could not accept this upload.');
	}

	// Align the server chunks with our metadata by chunk index.
	const serverChunks = [...data.chunks].sort(
		(a, b) => (a.chunk_index ?? a.index ?? 0) - (b.chunk_index ?? b.index ?? 0)
	);

	// 4. Encrypt (if enabled), PUT each chunk, then mark it complete.
	let done = 0;
	for (let i = 0; i < serverChunks.length; i++) {
		const meta = chunksMeta[i];
		const raw = new Uint8Array(await file.slice(meta.start, meta.end).arrayBuffer());
		const body = encrypt ? await encryptChunk(raw, key, meta._rawNonce) : raw;

		await axios.put(serverChunks[i].presigned_url, body, {
			headers: { 'Content-Type': 'application/octet-stream' }
		});
		await axios.post('/api/v1/public/request/mark-complete', {
			token,
			chunk_id: serverChunks[i].id
		});

		done++;
		if (onProgress) onProgress(Math.round((done / serverChunks.length) * 100));
	}

	return data.file;
}
