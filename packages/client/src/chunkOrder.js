/**
 * Where each encrypted chunk belongs in its file.
 *
 * Every chunk is sealed on its own with XChaCha20-Poly1305, so the cipher alone
 * does not stop whoever holds the chunk list (the server, its database, or
 * anyone who gets into either) from reordering the chunks, dropping the last
 * few or repeating one. Each chunk still decrypts and the file comes out wrong
 * with no error.
 *
 * So the uploader writes each chunk's position into its 24-byte nonce:
 *
 *   bytes  0..8   random, one value for the whole file
 *   bytes  8..16  FRAME_TAG, which marks a nonce laid out this way
 *   bytes 16..20  chunk index, big-endian
 *   bytes 20..24  chunk count, big-endian
 *
 * The nonce is an input to the cipher. A chunk only decrypts under the nonce it
 * was sealed with, so once it decrypts, the index and count in that nonce are
 * the uploader's. A downloader that checks them against the chunk's place in the
 * list catches reordering, truncation and duplication.
 *
 * This changes nothing for anything that only reads the nonce: clients that do
 * not know the layout still decrypt these files. Files uploaded before it have
 * fully random nonces with no FRAME_TAG and are read as they always were. Which
 * of the two a file is gets decided by its first chunk, and that chunk has to
 * decrypt before any of the file is used, so a forged answer gains nothing.
 *
 * packages/client/src/chunkOrder.js is a copy of this file and must stay
 * identical to it; a test compares the two.
 */

/** "silocat1": 8 bytes, so a random nonce carries it by chance with odds of 2^-64. */
export const FRAME_TAG = Uint8Array.of(0x73, 0x69, 0x6c, 0x6f, 0x63, 0x61, 0x74, 0x31);

const NONCE_BYTES = 24;
const MAX_U32 = 0xffffffff;

export class ChunkOrderError extends Error {
	constructor(message) {
		super(message);
		this.name = 'ChunkOrderError';
	}
}

/**
 * The nonce for chunk `index` of a file of `count` chunks.
 *
 * `fileSeed` is fresh random bytes, at least 8, drawn once per file and shared by
 * all of its chunks.
 *
 * @param {Uint8Array} fileSeed
 * @param {number} index
 * @param {number} count
 * @returns {Uint8Array}
 */
export function framedNonce(fileSeed, index, count) {
	if (!(fileSeed instanceof Uint8Array) || fileSeed.length < 8) {
		throw new Error('framedNonce needs at least 8 random bytes per file');
	}
	if (!Number.isInteger(count) || count < 1 || count > MAX_U32) {
		throw new Error(`Bad chunk count: ${count}`);
	}
	if (!Number.isInteger(index) || index < 0 || index >= count) {
		throw new Error(`Bad chunk index ${index} of ${count}`);
	}
	const nonce = new Uint8Array(NONCE_BYTES);
	nonce.set(fileSeed.subarray(0, 8), 0);
	nonce.set(FRAME_TAG, 8);
	const view = new DataView(nonce.buffer);
	view.setUint32(16, index, false);
	view.setUint32(20, count, false);
	return nonce;
}

/** @param {Uint8Array} nonce */
function isFramed(nonce) {
	if (nonce.length !== NONCE_BYTES) return false;
	for (let i = 0; i < FRAME_TAG.length; i++) {
		if (nonce[8 + i] !== FRAME_TAG[i]) return false;
	}
	return true;
}

/** @param {{ nonce?: string | null }} chunk @param {number} position */
function nonceOf(chunk, position) {
	if (!chunk?.nonce) throw new ChunkOrderError(`Chunk ${position} of this encrypted file has no nonce`);
	const raw = atob(chunk.nonce);
	const bytes = new Uint8Array(raw.length);
	for (let i = 0; i < raw.length; i++) bytes[i] = raw.charCodeAt(i);
	return bytes;
}

const ALTERED =
	'This file is incomplete or its chunks are out of order, so it was not saved';

/**
 * Checks chunks one at a time, in the order they are about to be joined.
 *
 * For a download that starts before the upload has finished: call `next` on
 * each chunk before using it and `done` once the last one is in.
 */
export function chunkOrderCheck() {
	let position = 0;
	/** @type {boolean | null} */
	let framed = null;
	/** @type {Uint8Array | null} */
	let seed = null;
	let count = 0;

	return {
		/** @param {{ nonce?: string | null }} chunk */
		next(chunk) {
			const nonce = nonceOf(chunk, position);
			if (framed === null) {
				framed = isFramed(nonce);
				if (framed) {
					seed = nonce.slice(0, 8);
					count = new DataView(nonce.buffer, nonce.byteOffset).getUint32(20, false);
				}
			}
			if (framed) {
				if (!isFramed(nonce)) throw new ChunkOrderError(ALTERED);
				const view = new DataView(nonce.buffer, nonce.byteOffset);
				const index = view.getUint32(16, false);
				const total = view.getUint32(20, false);
				const sameFile = seed !== null && seed.every((b, i) => b === nonce[i]);
				if (!sameFile || index !== position || total !== count) {
					throw new ChunkOrderError(ALTERED);
				}
			}
			position++;
		},

		/** Throws when the file is framed and chunks are missing from its end. */
		done() {
			if (framed && position !== count) throw new ChunkOrderError(ALTERED);
		}
	};
}

/**
 * Checks a complete chunk list before any of it is downloaded.
 *
 * @param {Array<{ nonce?: string | null }>} chunks
 */
export function assertChunkOrder(chunks) {
	const check = chunkOrderCheck();
	for (const chunk of chunks) check.next(chunk);
	check.done();
}
