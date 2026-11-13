// What silo.cat hands Google Analytics on its link pages. Run with `npm test`.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
	analyticsConfig,
	capabilityPageFields,
	capabilityRoute,
	needsFullLoad
} from '../src/lib/analytics.js';
import { track } from '../src/lib/track.js';

const O = 'https://silo.cat';
const DROP = '7f3c2a10-5b9e-4d1c-9a8e-2f6b1c0d4e5a';
const TOKEN = 'shTokQ7wErTy8uIoP3aSdFgHjKl';

/** Every string GA would receive, flattened, so a test can look for a secret in any of it. */
const sent = (/** @type {object} */ params) => JSON.stringify(params);

test('a drop reaches GA as its route, without the id or the file name', () => {
	const cfg = analyticsConfig({ href: `${O}/${DROP}` });
	assert.equal(cfg.page_location, `${O}/:id`);
	assert.equal(cfg.page_title, 'Secure download on SiloCat');
	assert.ok(!sent(cfg).includes(DROP));
});

test('share and request links reach GA as their routes', () => {
	assert.equal(analyticsConfig({ href: `${O}/s/${TOKEN}` }).page_location, `${O}/s/:token`);
	assert.equal(analyticsConfig({ href: `${O}/req/${TOKEN}` }).page_location, `${O}/req/:token`);
	assert.ok(!sent(analyticsConfig({ href: `${O}/req/${TOKEN}` })).includes(TOKEN));
});

test('query string and fragment on a link page are dropped with the token', () => {
	const cfg = analyticsConfig({ href: `${O}/s/${TOKEN}?utm_source=${TOKEN}#${TOKEN}` });
	assert.equal(cfg.page_location, `${O}/s/:token`);
	assert.ok(!sent(cfg).includes(TOKEN));
});

test('odd spellings of a link page are still link pages', () => {
	for (const path of [
		`/${DROP.toUpperCase()}`,
		`/${DROP}/`,
		`/${DROP}/extra`,
		`/s/${TOKEN}/extra`,
		`/S/${TOKEN}`,
		`/%73/${TOKEN}`,
		`//req/${TOKEN}`
	]) {
		assert.notEqual(capabilityRoute(path), null, path);
	}
});

test('ordinary pages are left for gtag.js to report as they are', () => {
	for (const href of [
		`${O}/`,
		`${O}/?ref=dl-drop-send`,
		`${O}/pricing`,
		`${O}/s`,
		`${O}/req`,
		`${O}/home/files?folder=${DROP}`,
		`${O}/auth/signin`
	]) {
		assert.deepEqual(analyticsConfig({ href, referrer: 'https://www.google.com/' }), {}, href);
	}
});

test('a page reached from a link page gets the route as its referrer', () => {
	for (const [from, route] of [
		[`${O}/${DROP}`, '/:id'],
		[`${O}/s/${TOKEN}`, '/s/:token'],
		[`${O}/req/${TOKEN}?x=1`, '/req/:token']
	]) {
		const cfg = analyticsConfig({ href: `${O}/?ref=dl-drop-send`, referrer: from });
		assert.deepEqual(cfg, { page_referrer: O + route }, from);
	}
});

test('other referrers are passed through untouched', () => {
	for (const referrer of [
		'',
		'https://web.whatsapp.com/',
		`${O}/pricing`,
		'https://example.com/s/abc'
	]) {
		assert.deepEqual(analyticsConfig({ href: `${O}/pricing`, referrer }), {}, referrer);
	}
});

test('a link page opened from another link page hides both', () => {
	const cfg = analyticsConfig({ href: `${O}/s/${TOKEN}`, referrer: `${O}/${DROP}` });
	assert.deepEqual(cfg, {
		page_location: `${O}/s/:token`,
		page_title: 'Secure download · Silocat',
		page_referrer: `${O}/:id`
	});
});

test('moving into or out of a link page in place is refused', () => {
	const u = (/** @type {string} */ p) => new URL(O + p);
	assert.equal(needsFullLoad(u(`/s/${TOKEN}`), u('/?ref=dl-share-footer')), true);
	assert.equal(needsFullLoad(u(`/${DROP}`), u('/pricing?ref=dl-drop-request')), true);
	assert.equal(needsFullLoad(u('/'), u(`/${DROP}`)), true);
	assert.equal(needsFullLoad(u('/home/requests'), u(`/req/${TOKEN}`)), true);
	assert.equal(needsFullLoad(u(`/s/${TOKEN}`), u(`/s/other${TOKEN}`)), true);
	assert.equal(needsFullLoad(null, u(`/s/${TOKEN}`)), true);
});

test('ordinary in-app navigation is left to the router', () => {
	const u = (/** @type {string} */ p) => new URL(O + p);
	assert.equal(needsFullLoad(u('/home'), u(`/home/files?folder=${DROP}`)), false);
	assert.equal(needsFullLoad(u('/'), u('/pricing')), false);
	// The same link page, only the hash moving: nothing new for gtag.js to see.
	assert.equal(needsFullLoad(u(`/s/${TOKEN}`), u(`/s/${TOKEN}#report`)), false);
});

test('track() names the route on a link page, and adds nothing elsewhere', () => {
	/** @type {any[]} */
	const calls = [];
	const g = /** @type {any} */ (globalThis);
	g.window = { gtag: (/** @type {any[]} */ ...args) => calls.push(args) };
	try {
		g.location = { href: `${O}/s/${TOKEN}` };
		track('download_complete', { surface: 'share', kind: 'file' });
		assert.deepEqual(calls.pop(), [
			'event',
			'download_complete',
			{
				surface: 'share',
				kind: 'file',
				page_location: `${O}/s/:token`,
				page_title: 'Secure download · Silocat'
			}
		]);

		g.location = { href: `${O}/pricing` };
		track('cta_click', { location: 'pricing_pro' });
		assert.deepEqual(calls.pop(), ['event', 'cta_click', { location: 'pricing_pro' }]);
	} finally {
		delete g.window;
		delete g.location;
	}
});

test('capabilityPageFields is empty off a link page', () => {
	assert.deepEqual(capabilityPageFields(`${O}/about`), {});
});
