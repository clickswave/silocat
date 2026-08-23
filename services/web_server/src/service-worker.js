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

	// Deliberately no Content-Length, for two concrete reasons.
	//
	// The exact figure is sum(chunk.size) over the chunks actually returned. That
	// is right for both cases: stored chunk sizes are PLAINTEXT lengths, and the
	// 16-byte Poly1305 tag on each encrypted chunk exists only in the R2 object,
	// not in what gets written here. But the sink has to be opened synchronously
	// inside the user gesture (showSaveFilePicker is unusable otherwise), which is
	// before the chunk list has been fetched, so that number is not known yet at
	// the moment this response has to be constructed.
	//
	// And `file.size` is not a safe substitute. fetch_chunks returns only chunks
	// with uploaded = true, so a partially uploaded file yields fewer bytes than
	// its recorded size. Today that saves a short file; with a Content-Length it
	// would instead fail the download outright, which is a worse outcome and a new
	// one, since the Blob path this replaces never depended on the size at all.
	//
	// The cost is the browser's own download UI showing a spinner rather than a
	// percentage. The app draws its own progress from chunk counts regardless.

	event.respondWith(new Response(entry.stream, { headers }));
});

/** Strip anything that cannot ride in a quoted HTTP header value. */
function asciiFallback(name) {
	return (name || 'download').replace(/[^\x20-\x7e]/g, '_').replace(/["\\]/g, '_');
}
