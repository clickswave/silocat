// Where the root layout loads Google Analytics, and how it keeps drop, share and
// request pages out of it. Run with `npm test`.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const O = 'https://silo.cat';
const DROP = '7f3c2a10-5b9e-4d1c-9a8e-2f6b1c0d4e5a';
const TOKEN = 'shTokQ7wErTy8uIoP3aSdFgHjKl';
const GA = 'G-TEST000';
const g = /** @type {any} */ (globalThis);

/** A location that records full page loads instead of making them. */
class FakeLocation {
	/** @param {string} href */
	constructor(href) {
		this.url = new URL(href);
		/** @type {string[][]} */
		this.loads = [];
	}
	get href() {
		return this.url.href;
	}
	set href(v) {
		this.loads.push(['href', String(v)]);
	}
	get pathname() {
		return this.url.pathname;
	}
	get hostname() {
		return this.url.hostname;
	}
	get origin() {
		return this.url.origin;
	}
	/** @param {string} v */
	assign(v) {
		this.loads.push(['assign', String(v)]);
	}
	/** @param {string} v */
	replace(v) {
		this.loads.push(['replace', String(v)]);
	}
}

let instance = 0;

/**
 * A fresh copy of $lib/analytics.js in a fake browser at `href`, started the
 * way the root layout starts it.
 * @param {string} href
 * @param {{ prod?: boolean, gaId?: string, gaHostname?: string, answer?: boolean }} [opts]
 */
async function start(href, opts = {}) {
	const { prod = true, gaId = GA, gaHostname, answer = true } = opts;
	const env = {
		location: new FakeLocation(href),
		/** @type {any[]} */ scripts: [],
		/** @type {Function[]} */ clicks: [],
		/** @type {any[][]} */ written: [],
		asked: 0,
		/** @type {any} */ mod: null,
		loaded: false
	};
	g.location = env.location;
	g.window = {
		addEventListener: (/** @type {string} */ type, /** @type {Function} */ fn, capture) => {
			if (type === 'click' && capture === true) env.clicks.push(fn);
		}
	};
	g.document = {
		head: { appendChild: (/** @type {any} */ el) => env.scripts.push(el) },
		createElement: (/** @type {string} */ tag) => ({ tagName: tag })
	};
	g.history = {
		pushState: (/** @type {any[]} */ ...args) => env.written.push(['push', ...args]),
		replaceState: (/** @type {any[]} */ ...args) => env.written.push(['replace', ...args])
	};
	env.mod = await import(`../src/lib/analytics.js?instance=${++instance}`);
	env.loaded = env.mod.startAnalytics({
		prod,
		gaId,
		gaHostname,
		confirm: () => {
			env.asked++;
			return answer;
		}
	});
	return env;
}

/** An anchor as the click handler sees it. */
function anchor(/** @type {string} */ href, /** @type {Record<string, string>} */ attrs = {}) {
	return {
		href: new URL(href, O).href,
		getAttribute: (/** @type {string} */ n) => (n in attrs ? attrs[n] : null),
		hasAttribute: (/** @type {string} */ n) => n in attrs
	};
}

/** Click an anchor (through a child element of it). True if the click was taken over. */
function click(env, a, extra = {}) {
	let taken = false;
	const ev = {
		button: 0,
		metaKey: false,
		ctrlKey: false,
		shiftKey: false,
		altKey: false,
		defaultPrevented: false,
		composedPath: () => [{ closest: (/** @type {string} */ sel) => (sel === 'a' ? a : null) }],
		preventDefault() {
			taken = true;
		},
		...extra
	};
	for (const fn of env.clicks) fn(ev);
	return taken;
}

test('gtag.js loads on an ordinary page of the configured host', async () => {
	const env = await start(`${O}/pricing`);
	assert.equal(env.loaded, true);
	assert.equal(env.scripts.length, 1);
	assert.equal(env.scripts[0].src, `https://www.googletagmanager.com/gtag/js?id=${GA}`);
	assert.equal(typeof g.window.gtag, 'function');
	assert.deepEqual(Array.from(g.window.dataLayer[1]), ['config', GA]);

	const own = await start('https://files.example.org/about', { gaHostname: 'files.example.org' });
	assert.equal(own.loaded, true);
});

test('gtag.js never loads on a link page, however the link is spelled', async () => {
	for (const path of [
		`/${DROP}`,
		`/${DROP}.`,
		`/${DROP})`,
		`/${DROP}%20`,
		`/${DROP}/extra`,
		`/s/${TOKEN}`,
		`/req/${TOKEN}`,
		`/S/${TOKEN}`
	]) {
		const env = await start(O + path);
		assert.equal(env.loaded, false, path);
		assert.deepEqual(env.scripts, [], path);
		assert.equal(g.window.gtag, undefined, path);
		// GA runs on the site, so leaving this page is a full load.
		assert.equal(env.mod.forcesFullLoad(env.location.url, new URL(`${O}/pricing`)), true, path);
	}
});

test('no GA and no full loads outside a production build on the configured host', async () => {
	for (const [href, opts] of /** @type {[string, any][]} */ ([
		[`${O}/pricing`, { prod: false }],
		[`${O}/pricing`, { gaId: '' }],
		['http://localhost:12001/pricing', {}],
		['https://staging.silo.cat/pricing', {}]
	])) {
		const env = await start(href, opts);
		assert.equal(env.loaded, false, href);
		assert.deepEqual(env.scripts, [], href);
		assert.deepEqual(env.clicks, [], href);
		assert.equal(env.mod.forcesFullLoad(new URL(href), new URL(`${O}/s/${TOKEN}`)), false, href);
		g.history.pushState({}, '', `/s/${TOKEN}`);
		assert.equal(env.written.length, 1, href);
	}
});

test('a click onto or off a link page loads it in full, before the router sees it', async () => {
	const env = await start(`${O}/pricing`);
	assert.equal(env.clicks.length, 1);

	assert.equal(click(env, anchor(`/s/${TOKEN}`)), true);
	assert.equal(click(env, anchor(`/${DROP}`)), true);
	assert.equal(click(env, anchor(`/req/${TOKEN}`)), true);
	assert.deepEqual(env.location.loads, [
		['href', `${O}/s/${TOKEN}`],
		['href', `${O}/${DROP}`],
		['href', `${O}/req/${TOKEN}`]
	]);
	assert.equal(env.asked, 3);

	// Left to the router, or to the browser.
	env.location.loads.length = 0;
	assert.equal(click(env, anchor('/about')), false);
	assert.equal(click(env, anchor('#plans')), false);
	assert.equal(click(env, anchor('https://example.com/s/x')), false);
	assert.equal(click(env, anchor(`/s/${TOKEN}`, { target: '_blank' })), false);
	assert.equal(click(env, anchor(`/s/${TOKEN}`, { download: '' })), false);
	assert.equal(click(env, anchor(`/s/${TOKEN}`), { ctrlKey: true }), false);
	assert.equal(click(env, anchor(`/s/${TOKEN}`), { metaKey: true }), false);
	assert.equal(click(env, anchor(`/s/${TOKEN}`), { button: 1 }), false);
	assert.equal(click(env, anchor(`/s/${TOKEN}`), { defaultPrevented: true }), false);
	assert.deepEqual(env.location.loads, []);

	// An explicit target=_self is the same tab.
	assert.equal(click(env, anchor(`/s/${TOKEN}`, { target: '_self' })), true);

	const share = await start(`${O}/s/${TOKEN}`);
	assert.equal(click(share, anchor('/?ref=dl-share-send')), true);
	assert.deepEqual(share.location.loads, [['href', `${O}/?ref=dl-share-send`]]);
	// Only the hash moving: the same page.
	assert.equal(click(share, anchor(`/s/${TOKEN}#report`)), false);
});

test('Cancel in the transfer prompt keeps the page and its transfer', async () => {
	const env = await start(`${O}/s/${TOKEN}`, { answer: false });
	assert.equal(click(env, anchor('/?ref=dl-share-footer')), true);
	assert.equal(env.asked, 1);
	assert.deepEqual(env.location.loads, []);
});

test('no link page URL is written into the history of a page where GA runs', async () => {
	const env = await start(`${O}/pricing`);
	g.history.pushState({ a: 1 }, '', `/s/${TOKEN}`);
	g.history.replaceState({ a: 2 }, '', new URL(`${O}/${DROP}`));
	g.history.pushState({ a: 3 }, '', `${O}/req/${TOKEN}?x=1`);
	assert.deepEqual(env.written, []);
	assert.deepEqual(env.location.loads, [
		['assign', `${O}/s/${TOKEN}`],
		['replace', `${O}/${DROP}`],
		['assign', `${O}/req/${TOKEN}?x=1`]
	]);
	assert.equal(g.window[`ga-disable-${GA}`], true);

	// Everything else goes through untouched.
	g.history.pushState({ b: 1 }, '', '/about');
	g.history.replaceState({ b: 2 }, '');
	g.history.replaceState({ b: 3 }, '', `${O}/pricing`);
	assert.deepEqual(env.written, [
		['push', { b: 1 }, '', '/about'],
		['replace', { b: 2 }, '', undefined],
		['replace', { b: 3 }, '', `${O}/pricing`]
	]);
	assert.equal(env.location.loads.length, 3);
});

test('the router moving onto or off a link page is cancelled for a full load', async () => {
	const nav = (/** @type {string} */ from, /** @type {string | null} */ to, willUnload = false) => {
		const n = {
			from: { url: new URL(O + from) },
			to: to === null ? null : { url: new URL(O + to) },
			willUnload,
			cancelled: false,
			cancel() {
				this.cancelled = true;
			}
		};
		return n;
	};

	const off = await start(`${O}/pricing`, { prod: false });
	const before = nav('/pricing', `/s/${TOKEN}`);
	off.mod.leaveLinkPagesInFull(before);
	assert.equal(before.cancelled, false, 'without GA the router keeps every move');

	const env = await start(`${O}/pricing`);
	const onto = nav('/pricing', `/s/${TOKEN}`);
	env.mod.leaveLinkPagesInFull(onto);
	assert.equal(onto.cancelled, true);
	const offOf = nav(`/${DROP}`, '/pricing?ref=dl-drop-request');
	env.mod.leaveLinkPagesInFull(offOf);
	assert.equal(offOf.cancelled, true);
	assert.deepEqual(env.location.loads, [
		['href', `${O}/s/${TOKEN}`],
		['href', `${O}/pricing?ref=dl-drop-request`]
	]);

	for (const n of [
		nav('/pricing', '/about'),
		nav('/pricing', null),
		nav('/pricing', `/s/${TOKEN}`, true)
	]) {
		env.mod.leaveLinkPagesInFull(n);
		assert.equal(n.cancelled, false);
	}
	assert.equal(env.location.loads.length, 2);

	const stay = await start(`${O}/s/${TOKEN}`, { answer: false });
	const away = nav(`/s/${TOKEN}`, '/');
	stay.mod.leaveLinkPagesInFull(away);
	assert.equal(away.cancelled, true);
	assert.deepEqual(stay.location.loads, []);
});

test('the root layout leaves both decisions to $lib/analytics.js', () => {
	const source = readFileSync(new URL('../src/routes/+layout.svelte', import.meta.url), 'utf8');
	const code = source
		.replace(/<!--[\s\S]*?-->/g, '')
		.replace(/\/\*[\s\S]*?\*\//g, '')
		.replace(/^\s*\/\/.*$/gm, '');
	assert.match(code, /\bbeforeNavigate\(leaveLinkPagesInFull\);/);
	const call = code.match(/\bstartAnalytics\(\{([^}]*)\}\);/);
	assert.ok(call, 'the layout does not start analytics');
	assert.match(call[1], /\bprod: import\.meta\.env\.PROD\b/);
	assert.match(call[1], /\bgaId: env\.PUBLIC_GA_ID\b/);
	assert.match(call[1], /\bgaHostname: env\.PUBLIC_GA_HOSTNAME\b/);
	assert.match(call[1], /\bconfirm: confirmFullLoad\b/);
	// Loading gtag.js anywhere else would skip the link page check.
	assert.doesNotMatch(code, /googletagmanager|dataLayer|gtag/);
});
