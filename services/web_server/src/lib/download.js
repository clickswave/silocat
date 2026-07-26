// Global download manager: chunked fetch -> (decrypt) -> reassemble -> save,
// with live per-file progress and cancellation. UI is DownloadToasts.svelte.
import { writable } from 'svelte/store';
import axios from 'axios';
import sodium from 'libsodium-wrappers-sumo';
import { decryptChunk, deriveKeyFromPassword } from '$lib/chacha.js';

// Each entry: { id, name, loaded, total, status, error, controller }
// status: 'active' | 'done' | 'error' | 'cancelled'
export const downloads = writable([]);

let nextId = 0;

function add(entry) {
	downloads.update((list) => [...list, entry]);
}
function patch(id, p) {
	downloads.update((list) => list.map((d) => (d.id === id ? { ...d, ...p } : d)));
}
function remove(id) {
	downloads.update((list) => list.filter((d) => d.id !== id));
}

function isCancel(e, controller) {
	return (
		controller.signal.aborted ||
		e?.code === 'ERR_CANCELED' ||
		e?.name === 'CanceledError' ||
		(axios.isCancel && axios.isCancel(e))
	);
}

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

	try {
		await sodium.ready;

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
			// argon2 key derivation is the slow part where the bar would otherwise sit at 0.
			patch(id, { phase: 'Deriving key…' });
			fileKey = await deriveKeyFromPassword(password, saltBytes);
		}

		patch(id, { phase: file.encrypted ? 'Downloading + decrypting…' : 'Downloading…' });

		const parts = [];
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
			parts.push(bytes);
			patch(id, { loaded });
		}

		const blob = new Blob(parts, { type: file.mime || 'application/octet-stream' });
		const url = URL.createObjectURL(blob);
		const a = document.createElement('a');
		a.href = url;
		a.download = file.name;
		document.body.appendChild(a);
		a.click();
		document.body.removeChild(a);
		URL.revokeObjectURL(url);

		patch(id, { status: 'done', loaded: total });
		setTimeout(() => remove(id), 4000);
	} catch (e) {
		if (isCancel(e, controller)) {
			patch(id, { status: 'cancelled' });
			setTimeout(() => remove(id), 2500);
		} else {
			console.error('[download]', e);
			patch(id, { status: 'error', error: e?.message || 'Download failed' });
			setTimeout(() => remove(id), 6000);
		}
	}
}

/**
 * Fetch + (decrypt) a file fully into an in-memory Blob (for inline preview).
 * Does NOT save to disk and does NOT register a DownloadToasts entry.
 * @param {{id:string,name:string,mime?:string,encrypted?:boolean}} file
 * @param {{ password?:string|null, chunksUrl?:string, signal?:AbortSignal, onProgress?:(loaded:number,total:number)=>void }} opts
 * @returns {Promise<Blob>}
 */
export async function fetchDecryptedBlob(
	file,
	{ password = null, chunksUrl = '/api/v1/sanctum/file/fetch-chunks', signal, onProgress } = {}
) {
	await sodium.ready;

	const chunksRes = await axios.post(chunksUrl, { file_id: file.id }, { signal });
	const chunks = chunksRes.data?.data?.chunks;
	if (!chunks || chunks.length === 0) throw new Error('No chunks found');

	const total = chunks.reduce((s, c) => s + (Number(c.size) || 0), 0) || Number(file.size) || 0;

	let fileKey = null;
	if (file.encrypted) {
		if (!password) throw new Error('Password required for encrypted file');
		if (!chunks[0].salt) throw new Error('Encrypted file is missing its salt');
		const saltBytes = Uint8Array.from(atob(chunks[0].salt), (c) => c.charCodeAt(0));
		fileKey = await deriveKeyFromPassword(password, saltBytes);
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

export function cancelDownload(id) {
	downloads.update((list) => {
		const d = list.find((x) => x.id === id);
		if (d?.controller) d.controller.abort();
		return list;
	});
}

export function dismissDownload(id) {
	remove(id);
}
