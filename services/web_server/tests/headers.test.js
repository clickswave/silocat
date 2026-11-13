// The security headers: _headers for the files Cloudflare Pages serves itself,
// and the same list for the pages the server renders. Run with `npm test`.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { SSR_SECURITY_HEADERS } from '../src/lib/server/securityHeaders.js';

/** The `Name: value` lines of _headers. */
function staticHeaders() {
	const text = readFileSync(new URL('../_headers', import.meta.url), 'utf8');
	/** @type {Record<string, string>} */
	const headers = {};
	for (const line of text.split('\n')) {
		const m = /^\s+([A-Za-z-]+):\s*(.*?)\s*$/.exec(line);
		if (m) headers[m[1]] = m[2];
	}
	return headers;
}

test('server-rendered pages get the same headers as _headers', () => {
	assert.deepEqual(SSR_SECURITY_HEADERS, staticHeaders());
});

test('no page hands the next one its path or query as the referrer', () => {
	// A drop, share or request link is a credential, and the next page may run GA.
	assert.equal(SSR_SECURITY_HEADERS['Referrer-Policy'], 'strict-origin');
});
