// Stands in for $lib/fileSink.js: keeps what would have been written to disk in
// globalThis.sinkWrites, and whether the download was saved or thrown away.
export const BLOB_SINK_LIMIT = 1024 * 1024 * 1024;

export function willStruggle() {
	return false;
}

export async function createFileSink() {
	const sink = { writes: [], closed: false, aborted: false };
	globalThis.lastSink = sink;
	return {
		write: async (bytes) => void sink.writes.push(new Uint8Array(bytes)),
		close: async () => void (sink.closed = true),
		abort: async () => void (sink.aborted = true)
	};
}
