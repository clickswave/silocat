// Stands in for $lib/network.js in tests that drive server routes: every call
// to api_switch goes to globalThis.fakeApi, which the test sets.
import { json } from '@sveltejs/kit';

const call = (method) => (path, ...rest) => globalThis.fakeApi[method](path, ...rest);

export const ApiServerClient = { get: call('get'), post: call('post') };

export function proxyError(err, _tag, fallback) {
	if (err?.response) return json(err.response.data, { status: err.response.status });
	return json({ error: fallback }, { status: 500 });
}
