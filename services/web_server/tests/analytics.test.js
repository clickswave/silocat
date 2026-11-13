// Where Google Analytics may run on silo.cat. Run with `npm test`.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
	forcesFullLoad,
	isLinkPage,
	needsFullLoad,
	separateLinkPages
} from '../src/lib/analytics.js';
import { track } from '../src/lib/track.js';

const O = 'https://silo.cat';
const DROP = '7f3c2a10-5b9e-4d1c-9a8e-2f6b1c0d4e5a';
const FOLDER = '0b9d6c4e-2a71-4f3e-8c55-91d2e7a4b6f0';
const TOKEN = 'shTokQ7wErTy8uIoP3aSdFgHjKl';
const u = (/** @type {string} */ p) => new URL(O + p);

test('drop, share and request pages are link pages, however they are spelled', () => {
	for (const path of [
		`/${DROP}`,
		`/s/${TOKEN}`,
		`/req/${TOKEN}`,
		`/${DROP.toUpperCase()}`,
		`/${DROP}/`,
		`/${DROP}/extra`,
		`/s/${TOKEN}/extra`,
		`/S/${TOKEN}`,
		`/%73/${TOKEN}`,
		`//req/${TOKEN}`
	]) {
		assert.equal(isLinkPage(path), true, path);
	}
});

test('other pages are not link pages', () => {
	for (const path of ['/', '/pricing', '/s', '/req', '/home/files', '/auth/signin', '/about']) {
		assert.equal(isLinkPage(path), false, path);
	}
});

test('moving into or out of a link page in place is refused', () => {
	assert.equal(needsFullLoad(u(`/s/${TOKEN}`), u('/?ref=dl-share-footer')), true);
	assert.equal(needsFullLoad(u(`/${DROP}`), u('/pricing?ref=dl-drop-request')), true);
	assert.equal(needsFullLoad(u('/'), u(`/${DROP}`)), true);
	assert.equal(needsFullLoad(u('/home/requests'), u(`/req/${TOKEN}`)), true);
	assert.equal(needsFullLoad(u(`/s/${TOKEN}`), u(`/s/other${TOKEN}`)), true);
	assert.equal(needsFullLoad(null, u(`/s/${TOKEN}`)), true);
});

test('ordinary in-app navigation is left to the router', () => {
	assert.equal(needsFullLoad(u('/home'), u(`/home/files?folder=${FOLDER}`)), false);
	assert.equal(needsFullLoad(u('/'), u('/pricing')), false);
	// The same link page, only the hash moving: no new page.
	assert.equal(needsFullLoad(u(`/s/${TOKEN}`), u(`/s/${TOKEN}#report`)), false);
});

test('the layout forces those full loads only once GA runs on the site', () => {
	assert.equal(forcesFullLoad(u(`/s/${TOKEN}`), u('/?ref=dl-share-footer')), false);
	separateLinkPages();
	assert.equal(forcesFullLoad(u(`/s/${TOKEN}`), u('/?ref=dl-share-footer')), true);
	assert.equal(forcesFullLoad(u('/'), u('/pricing')), false);
});

test('track() sends nothing from a link page, and other events as they are', () => {
	/** @type {any[]} */
	const calls = [];
	const g = /** @type {any} */ (globalThis);
	g.window = { gtag: (/** @type {any[]} */ ...args) => calls.push(args) };
	try {
		for (const path of [`/${DROP}`, `/s/${TOKEN}`, `/req/${TOKEN}`]) {
			g.location = { href: O + path, pathname: path };
			track('download_complete', { surface: 'share', kind: 'file' });
		}
		assert.deepEqual(calls, []);

		g.location = { href: `${O}/pricing`, pathname: '/pricing' };
		track('cta_click', { location: 'pricing_pro' });
		assert.deepEqual(calls, [['event', 'cta_click', { location: 'pricing_pro' }]]);
	} finally {
		delete g.window;
		delete g.location;
	}
});
