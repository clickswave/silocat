// The public request-link proxies must tell api_switch who the visitor is.
// Run with `npm test`.
//
// api_switch rate-limits the request info page per client IP before it looks
// the token up. The info proxy sent no X-Client-IP, so api_switch fell back to
// the address of the web_server's own hop, which is the same for every
// visitor: about 120 requests for any made-up token used up that one bucket,
// and every request link then showed "Too many attempts" to everyone.
import { test, beforeEach } from 'node:test';
import assert from 'node:assert/strict';
import * as nodeModule from 'node:module';

const here = new URL('./stubs/', import.meta.url).href;
const lib = new URL('../src/lib/', import.meta.url).href;
nodeModule.register(
	'data:text/javascript,' +
		encodeURIComponent(`
const here = ${JSON.stringify(here)};
const lib = ${JSON.stringify(lib)};
export function resolve(specifier, context, next) {
	if (specifier === '$lib/network.js') return { url: here + 'network.js', shortCircuit: true };
	if (specifier.startsWith('$lib/')) return { url: lib + specifier.slice(5), shortCircuit: true };
	return next(specifier, context);
}`)
);

const info = await import('../src/routes/api/v1/public/request/info/[token]/+server.js');
const upload = await import('../src/routes/api/v1/public/request/upload/+server.js');
const markComplete = await import('../src/routes/api/v1/public/request/mark-complete/+server.js');

/** The headers of every call that reached api_switch. */
let sent = [];

beforeEach(() => {
	sent = [];
	const ok = { status: 200, data: { status: 200, data: {} } };
	globalThis.fakeApi = {
		get: async (_path, opts) => {
			sent.push(opts?.headers ?? {});
			return ok;
		},
		post: async (_path, _body, opts) => {
			sent.push(opts?.headers ?? {});
			return ok;
		}
	};
});

/** A request event with only what these proxies read. @returns {any} */
function event(/** @type {string} */ ip, /** @type {object} */ body = {}) {
	return {
		params: { token: 'any-token' },
		request: new Request('http://localhost/', { method: 'POST', body: JSON.stringify(body) }),
		getClientAddress: () => ip
	};
}

test('the request info proxy forwards the visitor address', async () => {
	await info.GET(event('203.0.113.7'));
	assert.equal(sent.length, 1);
	assert.equal(sent[0]['X-Client-IP'], '203.0.113.7');
});

test('two visitors to the info page land in different buckets', async () => {
	await info.GET(event('203.0.113.7'));
	await info.GET(event('198.51.100.4'));
	assert.notEqual(sent[0]['X-Client-IP'], sent[1]['X-Client-IP']);
});

test('the request upload and mark-complete proxies forward the visitor address', async () => {
	await upload.POST(event('203.0.113.8', { token: 't' }));
	await markComplete.POST(event('203.0.113.9', { token: 't', chunk_id: 'c' }));
	assert.deepEqual(
		sent.map((h) => h['X-Client-IP']),
		['203.0.113.8', '203.0.113.9']
	);
});
