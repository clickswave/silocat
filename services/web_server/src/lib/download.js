// Global download manager: chunked fetch -> (decrypt) -> stream to disk, with
// live per-file progress and cancellation. UI is DownloadToasts.svelte.
//
// Two things used to make this unusable on real files:
//
//   * every chunk was collected into an array and handed to `new Blob()`, so
//     peak memory was roughly twice the file size. Uploads are allowed up to
//     20 GB anonymous / 50 GB authenticated; downloads died in the low
//     single-digit GB. Chunks now go to a sink that writes straight to disk.
//
//   * Argon2id key derivation and per-chunk decryption ran on the main thread,
//     freezing the tab solid for the duration. The phase label would be set to
//     "Deriving key…" and then block the thread that would have painted it.
//     Both now run in the shared crypto worker.
import { writable } from 'svelte/store';
import axios from 'axios';
import { deriveKey, decryptChunk } from '$lib/cryptoClient.js';
import { createFileSink, willStruggle, BLOB_SINK_LIMIT } from '$lib/fileSink.js';
import { holdTransfer } from '$lib/transferGuard.js';

/**
 * One in-flight or recently finished download, as DownloadToasts renders it.
 * @typedef {{
 *   id: number,
 *   name: string,
 *   loaded: number,
 *   total: number,
 *   status: 'active' | 'done' | 'error' | 'cancelled',
 *   phase?: string,
 *   error: string | null,
 *   controller: AbortController
 * }} DownloadEntry
 */

/** @type {import('svelte/store').Writable<DownloadEntry[]>} */
export const downloads = writable([]);

let nextId = 0;

/** @param {DownloadEntry} entry */
function add(entry) {
	downloads.update((list) => [...list, entry]);
}
/** @param {number} id @param {Partial<DownloadEntry>} p */
function patch(id, p) {
	downloads.update((list) => list.map((d) => (d.id === id ? { ...d, ...p } : d)));
}
/** @param {number} id */
function remove(id) {
	downloads.update((list) => list.filter((d) => d.id !== id));
}

/** @param {any} e @param {AbortController} controller */
function isCancel(e, controller) {
	return (
		controller.signal.aborted ||
		e?.code === 'ERR_CANCELED' ||
		e?.name === 'CanceledError' ||
		e?.name === 'SinkCancelled' ||
		(axios.isCancel && axios.isCancel(e))
	);
}

/** Above this, preview would download the whole file for nothing. Offer a save instead. */
export const PREVIEW_MAX_BYTES = 150 * 1024 * 1024;

/** True when this browser will have to buffer the whole file to download it. */
export { willStruggle, BLOB_SINK_LIMIT };

/**
 * Download a file with progress + cancel.
 * @param {{id:string,name:string,mime?:string,size?:number,encrypted?:boolean}} file
 * @param {{ password?:string|null, chunksUrl?:string }} opts
 */
export async function downloadFile(file, { password = null, chunksUrl = '/api/v1/sanctum/file/fetch-chunks' } = {}) {
	const id = ++nextId;
	const controller = new AbortController();
	add({
		id,
		name: file.name,
		loaded: 0,
		total: Number(file.size) || 0,
		status: 'active',
		phase: 'Preparing…',
		error: null,
		controller
	});

	let sink = null;
	// Claim the navigation guard for the whole transfer, released in `finally` so
	// an error cannot leave the app permanently prompting on every navigation.
	const release = holdTransfer();

	try {
		// The sink is opened FIRST, before any await.
		//
		// showSaveFilePicker() is only callable while the browser still considers
		// a user gesture to be in progress, and every await in between spends it.
		// With the chunk fetch and the Argon2id derivation ahead of it, Chromium
		// rejects the call with a SecurityError, fileSink quietly falls through to
		// the service-worker tier, and the best path in the app never runs. All the
		// metadata the picker needs (name, size, mime) is already on `file`, so
		// there is no reason to wait.
		//
		// It also means a dismissed save dialog costs nothing: no request has been
		// made yet.
		sink = await createFileSink(file.name, {
			size: Number(file.size) || 0,
			mime: file.mime
		});

		const chunksRes = await axios.post(chunksUrl, { file_id: file.id }, { signal: controller.signal });
		const chunks = chunksRes.data?.data?.chunks;
		if (!chunks || chunks.length === 0) throw new Error('No chunks found');

		// Total = sum of (server) chunk sizes: that's what we actually pull down.
		const total = chunks.reduce((s, c) => s + (Number(c.size) || 0), 0) || Number(file.size) || 0;
		patch(id, { total });

		let fileKey = null;
		if (file.encrypted) {
			if (!password) throw new Error('Password required for encrypted file');
			if (!chunks[0].salt) throw new Error('Encrypted file is missing its salt');
			const saltBytes = Uint8Array.from(atob(chunks[0].salt), (c) => c.charCodeAt(0));
			// Argon2id is the slow part where the bar would otherwise sit at 0. It
			// runs in the worker, so this label actually gets painted.
			patch(id, { phase: 'Deriving key…' });
			fileKey = await deriveKey(password, saltBytes);
		}

		patch(id, { phase: file.encrypted ? 'Downloading + decrypting…' : 'Downloading…' });

		let loaded = 0;
		for (const chunk of chunks) {
			const res = await axios.get(chunk.presigned_url, {
				responseType: 'arraybuffer',
				signal: controller.signal,
				onDownloadProgress: (e) => patch(id, { loaded: loaded + (e.loaded || 0) })
			});
			let bytes = new Uint8Array(res.data);
			loaded += bytes.byteLength;
			if (file.encrypted) {
				const nonceBytes = Uint8Array.from(atob(chunk.nonce), (c) => c.charCodeAt(0));
				bytes = await decryptChunk(bytes, fileKey, nonceBytes);
			}
			await sink.write(bytes);
			patch(id, { loaded });
		}

		patch(id, { phase: 'Saving…' });
		await sink.close();
		sink = null;

		patch(id, { status: 'done', loaded: total });
		setTimeout(() => remove(id), 4000);
	} catch (e) {
		await sink?.abort(e).catch(() => {});
		if (isCancel(e, controller)) {
			patch(id, { status: 'cancelled' });
			setTimeout(() => remove(id), 2500);
		} else {
			console.error('[download]', e);
			patch(id, { status: 'error', error: friendlyError(e, file) });
			setTimeout(() => remove(id), 6000);
		}
	} finally {
		release();
	}
}

/** Turn the failures people actually hit into something they can act on. */
/** @param {any} e @param {{encrypted?: boolean}} [file] */
function friendlyError(e, file) {
	const msg = e?.message || '';
	if (/decrypt|tag|verification/i.test(msg)) return 'Wrong password for this file';
	if (e?.name === 'QuotaExceededError' || /allocation|out of memory/i.test(msg)) {
		return 'This file is too large for your browser to download here';
	}
	if (file?.encrypted && /password/i.test(msg)) return msg;
	return msg || 'Download failed';
}

/**
 * Fetch + (decrypt) a file fully into an in-memory Blob (for inline preview).
 * Does NOT save to disk and does NOT register a DownloadToasts entry.
 *
 * This one genuinely needs the whole thing in memory, because a preview is
 * rendered from an object URL. That makes it the wrong tool for large files, so
 * callers must check `maxBytes` rather than letting someone preview 4 GB of
 * video into oblivion.
 *
 * @param {{id:string,name:string,mime?:string,size?:number,encrypted?:boolean}} file
 * @param {{ password?:string|null, chunksUrl?:string, signal?:AbortSignal, onProgress?:(loaded:number,total:number)=>void, maxBytes?:number }} opts
 * @returns {Promise<Blob>}
 */
export async function fetchDecryptedBlob(
	file,
	{
		password = null,
		chunksUrl = '/api/v1/sanctum/file/fetch-chunks',
		signal,
		onProgress,
		maxBytes = PREVIEW_MAX_BYTES
	} = {}
) {
	const chunksRes = await axios.post(chunksUrl, { file_id: file.id }, { signal });
	const chunks = chunksRes.data?.data?.chunks;
	if (!chunks || chunks.length === 0) throw new Error('No chunks found');

	const total = chunks.reduce((s, c) => s + (Number(c.size) || 0), 0) || Number(file.size) || 0;

	if (maxBytes && total > maxBytes) {
		throw new Error('TOO_LARGE_TO_PREVIEW');
	}

	let fileKey = null;
	if (file.encrypted) {
		if (!password) throw new Error('Password required for encrypted file');
		if (!chunks[0].salt) throw new Error('Encrypted file is missing its salt');
		const saltBytes = Uint8Array.from(atob(chunks[0].salt), (c) => c.charCodeAt(0));
		fileKey = await deriveKey(password, saltBytes);
	}

	const parts = [];
	let loaded = 0;
	for (const chunk of chunks) {
		const res = await axios.get(chunk.presigned_url, {
			responseType: 'arraybuffer',
			signal,
			onDownloadProgress: (e) => onProgress?.(loaded + (e.loaded || 0), total)
		});
		let bytes = new Uint8Array(res.data);
		loaded += bytes.byteLength;
		if (file.encrypted) {
			const nonceBytes = Uint8Array.from(atob(chunk.nonce), (c) => c.charCodeAt(0));
			bytes = await decryptChunk(bytes, fileKey, nonceBytes);
		}
		parts.push(bytes);
		onProgress?.(loaded, total);
	}

	return new Blob(parts, { type: file.mime || 'application/octet-stream' });
}

/** @param {number} id */
export function cancelDownload(id) {
	downloads.update((list) => {
		const d = list.find((x) => x.id === id);
		if (d?.controller) d.controller.abort();
		return list;
	});
}

/** @param {number} id */
export function dismissDownload(id) {
	remove(id);
}
