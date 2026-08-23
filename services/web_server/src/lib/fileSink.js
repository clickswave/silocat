/**
 * Somewhere to put a file as it arrives, without holding all of it in memory.
 *
 * Every download path used to collect decrypted chunks into an array and hand
 * the array to `new Blob()`. That means peak memory of roughly twice the file:
 * the parts plus the Blob's copy. With a 100 MB chunk size and an upload limit
 * of 20 GB anonymous / 50 GB authenticated, a download of any serious size
 * simply killed the tab. You could upload a file the app could never give back.
 *
 * A sink is `{ write(bytes), close(), abort(reason) }` and there are three
 * implementations, tried in order:
 *
 *   1. File System Access. `showSaveFilePicker` returns a real writable handle;
 *      chunks go straight to disk and the user picks the location up front.
 *      Chromium desktop. The best experience by a distance, and the only one
 *      where a paused/failed download leaves a resumable artifact.
 *
 *   2. Service worker stream. The SW answers a navigation with a ReadableStream
 *      and the browser's own downloader drains it to disk. Firefox, Safari,
 *      Android Chrome. Peak memory is one chunk.
 *
 *   3. Blob. The old behaviour, kept only as a last resort and used when neither
 *      of the above is available. `blobSinkLimit` reports the size above which
 *      this is expected to fail, so callers can warn before starting rather than
 *      after twenty minutes of downloading.
 *
 * Picking a strategy is async and may prompt (tier 1 opens a file dialog), so it
 * has to happen in the click handler's own task to keep the user gesture.
 */

/** Rough ceiling for the Blob fallback before the tab is likely to die. */
export const BLOB_SINK_LIMIT = 1024 * 1024 * 1024; // 1 GB

const STREAM_PREFIX = '/_stream/';

export function supportsFileSystemAccess() {
	return typeof window !== 'undefined' && typeof window.showSaveFilePicker === 'function';
}

/**
 * Which strategy `createFileSink` would choose, without side effects.
 * 'fs' | 'stream' | 'blob'
 */
export function sinkStrategy() {
	if (supportsFileSystemAccess()) return 'fs';
	if (typeof navigator !== 'undefined' && navigator.serviceWorker?.controller) return 'stream';
	return 'blob';
}

/** True when this download is big enough that the Blob fallback will likely fail. */
export function willStruggle(size) {
	return sinkStrategy() === 'blob' && Number(size) > BLOB_SINK_LIMIT;
}

/**
 * Open a sink for `filename`.
 * @param {string} filename
 * @param {{ size?: number, mime?: string }} opts
 */
export async function createFileSink(filename, { size = 0, mime = 'application/octet-stream' } = {}) {
	if (supportsFileSystemAccess()) {
		const sink = await fsAccessSink(filename, mime);
		if (sink) return sink;
		// User cancelled the save dialog: that is a cancellation, not a reason to
		// silently fall back to a different mechanism they did not ask for.
	}

	const streamed = await serviceWorkerSink(filename, size, mime);
	if (streamed) return streamed;

	return blobSink(filename, mime);
}

/** Thrown when the user dismisses the save dialog. */
export class SinkCancelled extends Error {
	constructor() {
		super('Download cancelled');
		this.name = 'SinkCancelled';
	}
}

// ---- 1. File System Access ------------------------------------------------

async function fsAccessSink(filename, mime) {
	const picker = window.showSaveFilePicker;
	if (!picker) return null;
	let handle;
	try {
		handle = await picker.call(window, {
			suggestedName: filename,
			types: [{ description: 'File', accept: { [mime || 'application/octet-stream']: [] } }]
		});
	} catch (e) {
		if (e?.name === 'AbortError') throw new SinkCancelled();
		return null; // not available after all (permissions policy, embedded, ...)
	}

	const writable = await handle.createWritable();
	return {
		strategy: 'fs',
		async write(bytes) {
			await writable.write(bytes);
		},
		async close() {
			await writable.close();
		},
		async abort() {
			try {
				await writable.abort();
			} catch {
				/* already closed */
			}
		}
	};
}

// ---- 2. Service worker stream ---------------------------------------------

async function serviceWorkerSink(filename, size, mime) {
	if (typeof navigator === 'undefined' || !navigator.serviceWorker) return null;

	const sw = navigator.serviceWorker.controller || (await waitForController());
	if (!sw) return null;

	// A TransformStream lets us write from here while the SW reads from the other
	// end, with the browser applying backpressure between them: if the disk is
	// slower than the network, write() waits instead of buffering.
	let ts;
	try {
		ts = new TransformStream();
	} catch {
		return null;
	}
	const writer = ts.writable.getWriter();

	const id = crypto.randomUUID();
	const url = await new Promise((resolve) => {
		const ch = new MessageChannel();
		ch.port1.onmessage = (e) => resolve(e.data?.url || null);
		try {
			sw.postMessage(
				{ type: 'stream-download', id, stream: ts.readable, filename, size, mime },
				[ts.readable, ch.port2]
			);
		} catch {
			resolve(null); // no transferable-stream support (Safari < 16.4)
		}
		setTimeout(() => resolve(null), 3000);
	});

	if (!url) {
		try {
			await writer.close();
		} catch {
			/* nothing consumed it */
		}
		return null;
	}

	// Navigating a hidden iframe starts the download without touching the page the
	// user is on.
	const frame = document.createElement('iframe');
	frame.hidden = true;
	frame.src = url;
	document.body.appendChild(frame);

	return {
		strategy: 'stream',
		async write(bytes) {
			await writer.ready;
			await writer.write(bytes);
		},
		async close() {
			await writer.close();
			setTimeout(() => frame.remove(), 2000);
		},
		async abort(reason) {
			try {
				await writer.abort(reason);
			} catch {
				/* already gone */
			}
			frame.remove();
		}
	};
}

function waitForController() {
	return new Promise((resolve) => {
		if (!navigator.serviceWorker) return resolve(null);
		navigator.serviceWorker.ready
			.then((reg) => resolve(navigator.serviceWorker.controller || reg.active || null))
			.catch(() => resolve(null));
		setTimeout(() => resolve(null), 2000);
	});
}

// ---- 3. Blob --------------------------------------------------------------

function blobSink(filename, mime) {
	const parts = [];
	return {
		strategy: 'blob',
		async write(bytes) {
			parts.push(bytes);
		},
		async close() {
			const blob = new Blob(parts, { type: mime || 'application/octet-stream' });
			parts.length = 0;
			const url = URL.createObjectURL(blob);
			const a = document.createElement('a');
			a.href = url;
			a.download = filename;
			document.body.appendChild(a);
			a.click();
			a.remove();
			// Revoking immediately can cancel the download in some browsers.
			setTimeout(() => URL.revokeObjectURL(url), 60_000);
		},
		async abort() {
			parts.length = 0;
		}
	};
}
