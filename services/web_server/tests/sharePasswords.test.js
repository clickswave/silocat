// The share page's two passwords. Run with `npm test`.
//
// A share can carry a link password (checked by the server on authorize and
// fetch-chunks) and, separately, files encrypted in the sender's browser (the
// decryption password, which must never leave the recipient's browser). The
// page used to keep one value for both, so on a password-protected link the
// decryption password went to the server on every request.
//
// These read the page itself: whatever reaches deriveKey must never appear in
// the body of a request to the share API, and each password has its own field.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { parse } from 'svelte/compiler';

const PAGE = new URL('../src/routes/s/[token]/+page.svelte', import.meta.url);
// The style block is SCSS, which the Svelte parser does not need to see.
const source = readFileSync(PAGE, 'utf8').replace(/<style[\s\S]*?<\/style>/, '');
const ast = /** @type {any} */ (parse(source, { modern: true }));
const script = ast.instance.content;

/** Depth-first walk over any ESTree or Svelte node, calling fn on each. */
function walk(/** @type {any} */ node, /** @type {(n: any) => void} */ fn) {
	if (!node || typeof node !== 'object') return;
	if (Array.isArray(node)) {
		for (const n of node) walk(n, fn);
		return;
	}
	if (typeof node.type === 'string') fn(node);
	for (const [key, value] of Object.entries(node)) {
		if (key === 'start' || key === 'end' || key === 'loc') continue;
		walk(value, fn);
	}
}

/** Identifier names an expression reads, leaving out non-computed property keys. */
function namesIn(/** @type {any} */ expr) {
	const names = new Set();
	walk(expr, (n) => {
		if (n.type === 'Property' && !n.computed && n.key?.type === 'Identifier') {
			n.key = { type: 'Skipped' };
		}
		if (n.type === 'MemberExpression' && !n.computed && n.property?.type === 'Identifier') {
			n.property = { type: 'Skipped' };
		}
		if (n.type === 'Identifier') names.add(n.name);
	});
	return names;
}

const shareCall = (/** @type {any} */ n) =>
	n.type === 'CallExpression' &&
	n.callee.type === 'MemberExpression' &&
	n.callee.object.name === 'axios' &&
	n.callee.property.name === 'post' &&
	/\/api\/v1\/public\/share\/(authorize|fetch-chunks)/.test(
		n.arguments[0]?.value ?? n.arguments[0]?.quasis?.map((q) => q.value.raw).join('') ?? ''
	);

/** Every name read by the body of a request to authorize or fetch-chunks. */
function sentToServer() {
	const sent = new Set();
	walk(script, (n) => {
		if (shareCall(n)) for (const name of namesIn(structuredClone(n.arguments[1]))) sent.add(name);
	});
	return sent;
}

/** Every name read by the password argument of deriveKey. */
function keyedFrom() {
	const keyed = new Set();
	let calls = 0;
	walk(script, (n) => {
		if (n.type === 'CallExpression' && n.callee.type === 'Identifier' && n.callee.name === 'deriveKey') {
			calls++;
			for (const name of namesIn(structuredClone(n.arguments[0]))) keyed.add(name);
		}
	});
	assert.ok(calls > 0, 'the page derives a key somewhere');
	return keyed;
}

/** Names declared at the top level of the page script, its component state. */
function topLevelNames() {
	const names = new Set();
	for (const stmt of script.body) {
		if (stmt.type === 'VariableDeclaration') {
			for (const d of stmt.declarations) if (d.id.type === 'Identifier') names.add(d.id.name);
		}
	}
	return names;
}

/** Inputs in the markup bound with bind:value, by the name they bind. */
function boundInputs() {
	const bound = new Map();
	walk(ast.fragment, (n) => {
		if (n.type !== 'RegularElement' || n.name !== 'input') return;
		for (const a of n.attributes) {
			if (a.type === 'BindDirective' && a.name === 'value' && a.expression.type === 'Identifier') {
				bound.set(a.expression.name, n);
			}
		}
	});
	return bound;
}

test('the decryption password never goes into a request to the share API', () => {
	const sent = sentToServer();
	const keyed = keyedFrom();
	assert.ok(sent.size > 0, 'the page sends authorize and fetch-chunks requests');
	const leaked = [...keyed].filter((name) => sent.has(name));
	assert.deepEqual(leaked, [], `sent to the server and used as the key: ${leaked.join(', ')}`);
});

test('the key comes from page state, not from a value handed in by a caller', () => {
	// A parameter would let any caller pass the link password, or the other way
	// round, without the names above ever meeting.
	const state = topLevelNames();
	for (const name of keyedFrom()) {
		if (name === 'saltBytes') continue;
		assert.ok(state.has(name), `deriveKey reads ${name}, which is not page state`);
	}
});

test('the link password and the decryption password each have a field of their own', () => {
	const bound = boundInputs();
	const sent = sentToServer();
	const keyed = [...keyedFrom()].filter((n) => n !== 'saltBytes');
	const linkFields = [...bound.keys()].filter((n) => sent.has(n));
	const keyFields = [...bound.keys()].filter((n) => keyed.includes(n));
	assert.equal(linkFields.length, 1, `one field feeds the server: ${linkFields.join(', ')}`);
	assert.equal(keyFields.length, 1, `one field feeds the key: ${keyFields.join(', ')}`);
	assert.notEqual(linkFields[0], keyFields[0]);
});
