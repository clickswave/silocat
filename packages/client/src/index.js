/**
 * Silocat client.
 *
 * Silocat is zero-knowledge: the server holds ciphertext and never sees your
 * password or the key derived from it. That is a real constraint on any client,
 * not a detail this library hides. What it does hide is the *mechanics* of
 * getting it right, because those are easy to get subtly wrong in a way that
 * produces files nobody can ever open.
 *
 *   import { Silocat } from '@silocat/client';
 *
 *   const silo = new Silocat({ apiKey: process.env.SILOCAT_API_KEY });
 *
 *   const file = await silo.upload(bytes, {
 *     name: 'contract.pdf',
 *     password: 'correct-horse-battery-staple'
 *   });
 *
 *   const link = await silo.share(file.id, { type: 'public', expiresInDays: 7 });
 *
 * Lose the password and the file is gone. There is no recovery path, by design.
 */
import {
	ready,
	deriveKey,
	randomSalt,
	randomNonce,
	encryptChunk,
	decryptChunk,
	sha256Hex,
	b64
} from './crypto.js';

/** Matches the web app. Chunks are uploaded individually to presigned URLs. */
const CHUNK_SIZE = 100 * 1024 * 1024;

export class SilocatError extends Error {
	constructor(message, { status, body } = {}) {
		super(message);
		this.name = 'SilocatError';
		this.status = status;
		this.body = body;
	}
}

export class Silocat {
	/**
	 * @param {object} opts
	 * @param {string} opts.apiKey  From Settings → API in the web app.
	 * @param {string} [opts.baseUrl] Override for self-hosted instances.
	 * @param {typeof fetch} [opts.fetch] Inject a fetch (tests, proxies).
	 */
	constructor({ apiKey, baseUrl = 'https://silo.cat', fetch: f } = {}) {
		if (!apiKey) throw new SilocatError('An apiKey is required');
		this.apiKey = apiKey;
		this.baseUrl = baseUrl.replace(/\/+$/, '');
		this._fetch = f ?? globalThis.fetch;
		if (!this._fetch) {
			throw new SilocatError('No fetch available; pass one via { fetch }');
		}
	}

	async #call(path, { method = 'GET', body, params } = {}) {
		const url = new URL(this.baseUrl + path);
		for (const [k, v] of Object.entries(params ?? {})) {
			if (v !== undefined && v !== null) url.searchParams.set(k, String(v));
		}

		const res = await this._fetch(url, {
			method,
			headers: {
				'X-Api-Key': this.apiKey,
				...(body ? { 'Content-Type': 'application/json' } : {})
			},
			body: body ? JSON.stringify(body) : undefined
		});

		const text = await res.text();
		let parsed;
		try {
			parsed = text ? JSON.parse(text) : {};
		} catch {
			parsed = { raw: text };
		}

		if (!res.ok) {
			throw new SilocatError(
				parsed?.message || parsed?.error || `Request failed (${res.status})`,
				{ status: res.status, body: parsed }
			);
		}
		return parsed;
	}

	// ---- account ----------------------------------------------------------

	/** Bytes used and available. */
	async storage() {
		const r = await this.#call('/api/v1/sanctum/user/storage');
		const s = r?.success ?? r?.data ?? {};
		return { used: s.used ?? 0, total: s.total ?? 0, free: s.free ?? 0 };
	}

	// ---- browsing ---------------------------------------------------------

	/** Files in a folder, or at the root when `folderId` is omitted. */
	async listFiles({ folderId = null, starred, shared } = {}) {
		const r = await this.#call('/api/v1/sanctum/file/list', {
			params: { folder_id: folderId, starred, shared }
		});
		return r?.data?.files ?? [];
	}

	async listFolders({ parentId = null } = {}) {
		const r = await this.#call('/api/v1/sanctum/folder/list', {
			method: 'POST',
			body: { parent_id: parentId }
		});
		return r?.data?.folders ?? [];
	}

	async createFolder(name, { parentId = null } = {}) {
		const r = await this.#call('/api/v1/sanctum/folder/create', {
			method: 'POST',
			body: { name, parent_id: parentId }
		});
		return r?.data?.folder ?? r?.data;
	}

	// ---- upload -----------------------------------------------------------

	/**
	 * Encrypt and upload. The plaintext never leaves this process.
	 *
	 * @param {Uint8Array|ArrayBuffer|Blob} data
	 * @param {object} opts
	 * @param {string} opts.name      Filename to store.
	 * @param {string} [opts.password] Omit to upload unencrypted (readable by the server).
	 * @param {string} [opts.mime]
	 * @param {string|null} [opts.folderId]
	 * @param {(p:{uploadedBytes:number,totalBytes:number,ratio:number}) => void} [opts.onProgress]
	 */
	async upload(data, { name, password, mime, folderId = null, onProgress } = {}) {
		await ready();
		if (!name) throw new SilocatError('upload() needs a name');

		const bytes = await toBytes(data);
		const encrypted = Boolean(password);

		// One salt per file, one nonce per chunk. Reusing a nonce under the same
		// key would break the cipher's guarantees outright.
		const salt = encrypted ? await randomSalt() : null;
		const key = encrypted ? await deriveKey(password, salt) : null;

		const checksum = await sha256Hex(bytes);
		const total = bytes.byteLength;
		const chunkCount = Math.max(1, Math.ceil(total / CHUNK_SIZE));

		const chunks = [];
		for (let i = 0; i < chunkCount; i++) {
			const start = i * CHUNK_SIZE;
			const end = Math.min(start + CHUNK_SIZE, total);
			const nonce = encrypted ? await randomNonce() : null;
			chunks.push({
				start,
				end,
				size: end - start,
				checksum: 'pending',
				salt: salt ? b64.encode(salt) : null,
				nonce: nonce ? b64.encode(nonce) : null,
				_nonce: nonce
			});
		}

		// Register the file first: the server hands back a presigned PUT per chunk.
		const created = await this.#call('/api/v1/sanctum/file', {
			method: 'POST',
			body: {
				storage_type: 'sanctum',
				file_encrypted: encrypted,
				file_name: name,
				file_mime: mime || 'application/octet-stream',
				file_size: total,
				chunks: chunks.map(({ _nonce, ...c }) => c),
				sha256_checksum: checksum,
				blake3_checksum: '',
				public_access: !encrypted,
				folder_id: folderId
			}
		});

		const file = created?.data?.file;
		const slots = created?.data?.chunks ?? [];
		if (!file?.id) throw new SilocatError('Server did not return a file id', { body: created });

		let uploadedBytes = 0;
		for (let i = 0; i < chunks.length; i++) {
			const meta = chunks[i];
			const slot = slots[i];
			if (!slot?.presigned_url) {
				throw new SilocatError(`Missing upload slot for chunk ${i}`, { body: created });
			}

			const plain = bytes.subarray(meta.start, meta.end);
			const payload = encrypted ? await encryptChunk(plain, key, meta._nonce) : plain;

			await this.#putChunk(slot.presigned_url, payload);
			await this.#call('/api/v1/sanctum/file/mark-chunk-complete', {
				method: 'POST',
				body: { chunk_id: slot.id }
			});

			uploadedBytes += meta.size;
			onProgress?.({ uploadedBytes, totalBytes: total, ratio: uploadedBytes / total });
		}

		return { ...file, encrypted };
	}

	/** Object storage occasionally 500s mid-upload; a chunk is safe to retry. */
	async #putChunk(url, payload, attempts = 3) {
		let lastError;
		for (let attempt = 1; attempt <= attempts; attempt++) {
			try {
				const res = await this._fetch(url, {
					method: 'PUT',
					headers: { 'Content-Type': 'application/octet-stream' },
					body: payload
				});
				if (!res.ok) throw new SilocatError(`Chunk upload failed (${res.status})`, { status: res.status });
				return;
			} catch (err) {
				lastError = err;
				if (attempt < attempts) await sleep(1000 * attempt);
			}
		}
		throw lastError;
	}

	// ---- download ---------------------------------------------------------

	/**
	 * Fetch and decrypt. Returns the plaintext bytes.
	 *
	 * @param {string} fileId
	 * @param {object} [opts]
	 * @param {string} [opts.password] Required if the file was encrypted.
	 */
	async download(fileId, { password, onProgress } = {}) {
		await ready();

		const r = await this.#call('/api/v1/sanctum/file/fetch-chunks', {
			method: 'POST',
			body: { file_id: fileId }
		});
		const chunks = r?.data?.chunks ?? [];
		if (!chunks.length) throw new SilocatError('No chunks returned for that file');

		const encrypted = chunks.some((c) => c.nonce);
		if (encrypted && !password) {
			throw new SilocatError('This file is encrypted; pass { password }');
		}

		// The salt rides with the chunk metadata: same salt, same password, same key.
		let key = null;
		if (encrypted) {
			const salt = b64.decode(chunks[0].salt);
			key = await deriveKey(password, salt);
		}

		const parts = [];
		let done = 0;
		for (const chunk of chunks) {
			const res = await this._fetch(chunk.presigned_url);
			if (!res.ok) throw new SilocatError(`Chunk download failed (${res.status})`, { status: res.status });
			const raw = new Uint8Array(await res.arrayBuffer());

			let plain;
			if (encrypted) {
				try {
					plain = await decryptChunk(raw, key, b64.decode(chunk.nonce));
				} catch {
					// libsodium fails the auth tag for both a wrong password and a
					// tampered blob; the former is overwhelmingly likelier.
					throw new SilocatError('Decryption failed: wrong password, or the file was altered');
				}
			} else {
				plain = raw;
			}

			parts.push(plain);
			done += plain.byteLength;
			onProgress?.({ downloadedBytes: done });
		}

		return concat(parts);
	}

	// ---- sharing ----------------------------------------------------------

	/**
	 * Turn sharing on and get a link.
	 *
	 * A share link never carries the decryption password. Send that separately,
	 * or the link alone is enough to read the file.
	 *
	 * @param {string} fileId
	 * @param {object} [opts]
	 * @param {'public'|'once'|'off'} [opts.type]
	 */
	async share(fileId, { type = 'public' } = {}) {
		const r = await this.#call('/api/v1/sanctum/file/share/toggle', {
			method: 'POST',
			body: { file_id: fileId, share_type: type }
		});
		const d = r?.success?.data ?? r?.data ?? {};
		const token = d.share_token ?? d.token;
		return { ...d, token, url: token ? `${this.baseUrl}/s/${token}` : null };
	}

	async unshare(fileId) {
		return this.share(fileId, { type: 'off' });
	}

	// ---- lifecycle --------------------------------------------------------

	/** Moves to trash. Trashed items self-delete after the retention window. */
	async trash(fileId) {
		await this.#call('/api/v1/sanctum/file/delete', {
			method: 'POST',
			body: { file_id: fileId }
		});
		return { trashed: true };
	}

	/** Irreversible. */
	async deleteForever(fileId) {
		await this.#call('/api/v1/sanctum/file/permanent-delete', {
			method: 'POST',
			body: { file_id: fileId }
		});
		return { deleted: true };
	}

	async restore(fileId) {
		await this.#call('/api/v1/sanctum/file/restore', {
			method: 'POST',
			body: { file_id: fileId }
		});
		return { restored: true };
	}
}

// ---- helpers --------------------------------------------------------------

async function toBytes(data) {
	if (data instanceof Uint8Array) return data;
	if (data instanceof ArrayBuffer) return new Uint8Array(data);
	if (typeof Blob !== 'undefined' && data instanceof Blob) {
		return new Uint8Array(await data.arrayBuffer());
	}
	throw new SilocatError('upload() expects a Uint8Array, ArrayBuffer or Blob');
}

function concat(parts) {
	const total = parts.reduce((n, p) => n + p.byteLength, 0);
	const out = new Uint8Array(total);
	let offset = 0;
	for (const p of parts) {
		out.set(p, offset);
		offset += p.byteLength;
	}
	return out;
}

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

export default Silocat;
