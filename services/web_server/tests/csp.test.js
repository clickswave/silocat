// The Content-Security-Policy that svelte.config.js hands SvelteKit.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import config from '../svelte.config.js';

const directives = /** @type {Record<string, string[]>} */ (config.kit?.csp?.directives ?? {});

test('script-src names the gtag.js loader by path, not the Google Tag Manager host', () => {
	const scriptSrc = directives['script-src'] ?? [];
	assert.ok(scriptSrc.includes('https://www.googletagmanager.com/gtag/js'), scriptSrc.join(' '));
	for (const source of scriptSrc) {
		assert.ok(!source.includes('*'), `wildcard in script-src: ${source}`);
	}
});
