/**
 * Download-stream relay.
 *
 * The only reason this service worker exists: browsers without the File System
 * Access API have no way to write a file to disk incrementally from script. The
 * app's downloads are chunked, decrypted one chunk at a time, and can be tens of
 * gigabytes, so buffering the whole thing to produce a Blob is not an option (it
 * was, until now, and it is why large downloads died).
 *
 * The way around it is the one the ecosystem settled on: a service worker
 * answers a request with a ReadableStream, the page navigates to that request,
 * and the browser's own downloader consumes the stream and writes it to disk.
 * Peak memory is one chunk, and the user gets a real progress bar in their
 * browser's download UI.
 *
 * Deliberately inert otherwise. It registers no caches, claims no navigations,
 * and passes every request it does not own straight through, so adding it does
 * not change how anything else on the site loads or updates. Everything it
 * handles lives under STREAM_PREFIX.
 */

const STREAM_PREFIX = '/_stream/';

// id -> { stream, filename, size, mime }. Populated by a postMessage from the
// page, consumed by the fetch handler, deleted on consumption.
const pending = new Map();

self.addEventListener('install', () => self.skipWaiting());
self.addEventListener('activate', (event) => event.waitUntil(self.clients.claim()));

self.addEventListener('message', (event) => {
	const data = event.data;
	if (!data || data.type !== 'stream-download') return;

	pending.set(data.id, {
		stream: data.stream,
		filename: data.filename,
		size: data.size,
		mime: data.mime
	});

	// Hand the page back the URL to navigate to. It cannot be guessed from
	// outside because the id is a random uuid held only by this page and this
	// worker.
	event.ports[0]?.postMessage({ url: new URL(STREAM_PREFIX + data.id, self.registration.scope).href });

	// A page that dies before navigating would leak its stream forever.
	setTimeout(() => pending.delete(data.id), 60_000);
});

self.addEventListener('fetch', (event) => {
	const url = new URL(event.request.url);
	if (url.origin !== self.location.origin || !url.pathname.startsWith(STREAM_PREFIX)) {
		return; // not ours: let the network handle it exactly as before
	}

	const id = url.pathname.slice(STREAM_PREFIX.length);
	const entry = pending.get(id);
	if (!entry) {
		event.respondWith(new Response('Download expired', { status: 404 }));
		return;
	}
	pending.delete(id);

	const headers = new Headers({
		'Content-Type': entry.mime || 'application/octet-stream',
		// RFC 5987 form so non-ASCII names survive. The plain `filename=` is kept
		// alongside it for older parsers.
		'Content-Disposition':
			`attachment; filename="${asciiFallback(entry.filename)}"; ` +
			`filename*=UTF-8''${encodeURIComponent(entry.filename)}`,
		'Content-Security-Policy': "default-src 'none'",
		'X-Content-Type-Options': 'nosniff'
	});

	// Deliberately no Content-Length.
	//
	// It would give the browser a real progress bar rather than a spinner, but it
	// also makes the download fail outright if the byte count we declare and the
	// bytes we deliver ever disagree. The sizes come from file metadata recorded
	// at upload; for an encrypted file that is the plaintext length, and the
	// decrypted stream should match it exactly. "Should" is doing work in that
	// sentence, and the Blob path this replaces never cared whether it was true,
	// so setting the header would introduce a new way for downloads to break. The
	// app draws its own progress from chunk counts anyway.

	event.respondWith(new Response(entry.stream, { headers }));
});

/** Strip anything that cannot ride in a quoted HTTP header value. */
function asciiFallback(name) {
	return (name || 'download').replace(/[^\x20-\x7e]/g, '_').replace(/["\\]/g, '_');
}
