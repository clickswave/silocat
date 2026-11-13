// The transfer guard against the root layout's full page loads. Run with `npm test`.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import './sveltekit-stub.js';

const { confirmFullLoad, guardNavigation, holdTransfer } =
	await import('../src/lib/transferGuard.js');
const { separateLinkPages } = await import('../src/lib/analytics.js');

const O = 'https://silo.cat';
const SHARE = new URL(`${O}/s/shTokQ7wErTy8uIoP3aSdFgHjKl`);
const PROMPT = 'Your download is still running. Leaving this page will cancel it.';
const g = /** @type {any} */ (globalThis);

/** Every confirm() the page shows, and the visitor's answer to the next one. */
const asked = /** @type {string[]} */ ([]);
let answer = true;
g.confirm = (/** @type {string} */ text) => {
	asked.push(text);
	return answer;
};

// A window that keeps its beforeunload listeners, so a test can fire them.
const unloadListeners = /** @type {Function[]} */ ([]);
g.window = {
	addEventListener: (/** @type {string} */ type, /** @type {Function} */ fn) => {
		if (type === 'beforeunload') unloadListeners.push(fn);
	},
	removeEventListener: (/** @type {string} */ type, /** @type {Function} */ fn) => {
		const i = unloadListeners.indexOf(fn);
		if (type === 'beforeunload' && i >= 0) unloadListeners.splice(i, 1);
	}
};

/** Fire beforeunload, as a full page load does. True when the browser would ask. */
function unloadAsks() {
	let asks = false;
	const event = { returnValue: undefined, preventDefault: () => (asks = true) };
	for (const fn of unloadListeners) fn(event);
	return asks;
}

/** An in-app navigation, through every beforeNavigate callback. True if one cancelled it. */
function navigate(/** @type {URL} */ from, /** @type {URL} */ to) {
	let cancelled = false;
	const nav = {
		from: { url: from },
		to: { url: to },
		willUnload: false,
		cancel: () => (cancelled = true)
	};
	for (const fn of g.beforeNavigateCallbacks) fn(nav);
	return cancelled;
}

guardNavigation(PROMPT);
// GA runs on this site: leaving a link page is a full page load.
separateLinkPages();

test('with nothing running, a full page load goes ahead unasked', () => {
	asked.length = 0;
	assert.equal(confirmFullLoad(), true);
	assert.deepEqual(asked, []);
	assert.equal(unloadAsks(), false);
});

test('mid-download, leaving a share page asks once, in the words of the page', () => {
	const release = holdTransfer();
	try {
		asked.length = 0;
		answer = true;
		// The guard leaves this move to the layout, which loads the next page in full.
		assert.equal(navigate(SHARE, new URL(`${O}/?ref=dl-share-footer`)), false);
		assert.deepEqual(asked, []);
		assert.equal(confirmFullLoad(), true);
		assert.deepEqual(asked, [PROMPT]);
		// The load that follows does not ask again...
		assert.equal(unloadAsks(), false);
		// ...but only that one.
		assert.equal(unloadAsks(), true);
	} finally {
		release();
	}
});

test('Cancel keeps the download, and closing the tab still asks', () => {
	const release = holdTransfer();
	try {
		asked.length = 0;
		answer = false;
		assert.equal(confirmFullLoad(), false);
		assert.deepEqual(asked, [PROMPT]);
		assert.equal(unloadAsks(), true);
	} finally {
		release();
		answer = true;
	}
});

test('a transfer that starts after an agreed leave is guarded again', () => {
	const first = holdTransfer();
	try {
		answer = true;
		assert.equal(confirmFullLoad(), true);
		const second = holdTransfer();
		try {
			assert.equal(unloadAsks(), true);
		} finally {
			second();
		}
	} finally {
		first();
	}
});

test('an in-app move between other pages is still asked by the guard', () => {
	const release = holdTransfer();
	try {
		asked.length = 0;
		answer = false;
		assert.equal(navigate(new URL(`${O}/`), new URL(`${O}/pricing`)), true);
		assert.deepEqual(asked, [PROMPT]);
	} finally {
		release();
		answer = true;
	}
});
