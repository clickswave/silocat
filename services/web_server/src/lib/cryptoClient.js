/**
 * Single client for the crypto worker.
 *
 * Argon2id at libsodium's MODERATE limits is 256 MiB and roughly a second of
 * solid compute; XChaCha20-Poly1305 over a chunk is the same order of work again.
 * Run either on the main thread and the tab stops painting: no spinner movement,
 * no cancel button, no scrolling. The upload path already knew this and drove a
 * worker, but it owned that worker privately inside the Files page, so the
 * download path (lib/download.js) and the file-request path (lib/requestUpload.js)
 * both imported chacha.js directly and blocked the UI. Worse, the download
 * progress label would set itself to "Deriving key…" and then block the very
 * thread that would have painted it.
 *
 * One module, one worker, shared by every caller.
 *
 * The KDF here MUST stay byte-for-byte identical to deriveKeyFromPassword in
 * chacha.js. chacha.js remains the reference implementation and is still used by
 * anything that cannot reach a worker; if the two ever diverge, files encrypted
 * by one become undecryptable by the other, which is permanent data loss.
 */
import CryptoWorker from '$lib/workers/crypto.worker.js?worker';

let worker = null;
let callbacks = new Map();
let seq = 0;

function ensureWorker() {
	if (worker) return worker;
	worker = new CryptoWorker();
	worker.onmessage = (e) => {
		const { id, status, result, error } = e.data;
		if (!id || !callbacks.has(id)) return; // the worker's own 'ready' ping
		const { resolve, reject } = callbacks.get(id);
		callbacks.delete(id);
		if (status === 'success') resolve(result);
		else reject(new Error(error));
	};
	worker.onerror = (e) => {
		// A worker that died owes answers to everything still pending; without
		// this they hang forever.
		rejectAll(new Error(e.message || 'Crypto worker failed'));
		worker = null;
	};
	return worker;
}

function rejectAll(err) {
	for (const { reject } of callbacks.values()) reject(err);
	callbacks.clear();
}

function call(type, payload, transferables = []) {
	const w = ensureWorker();
	return new Promise((resolve, reject) => {
		const id = `${++seq}`;
		callbacks.set(id, { resolve, reject });
		w.postMessage({ id, type, payload }, transferables);
	});
}

/**
 * Stop the worker mid-call.
 *
 * A long hash or KDF is one message round trip, so a flag checked on return does
 * nothing until it completes: on a large file that is the entire wait.
 * Terminating is the only way to interrupt it. Pending calls are rejected first
 * because a terminated worker will never answer them. A fresh worker is created
 * lazily on the next call.
 */
export function terminateCrypto(reason = 'Cancelled') {
	rejectAll(new Error(reason));
	worker?.terminate();
	worker = null;
}

/** SHA-256 of a File, read in slices so a large file is never fully resident. */
export function hashFile(file, chunkSize) {
	return call('hashFile', { file, chunkSize });
}

/** Argon2id password -> 32-byte key. `salt` is a 16-byte Uint8Array. */
export function deriveKey(password, salt) {
	return call('deriveKey', { password, salt });
}

/** Encrypt one chunk. `chunk` is a Uint8Array whose buffer is transferred. */
export function encryptChunk(chunk, key, nonce) {
	return call('encryptChunk', { chunk, key, nonce }, [chunk.buffer]);
}

/** Decrypt one chunk. Rejects when the tag does not verify (wrong password). */
export function decryptChunk(chunk, key, nonce) {
	return call('decryptChunk', { chunk, key, nonce }, [chunk.buffer]);
}
