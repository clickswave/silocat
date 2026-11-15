// What silo.cat says about encryption. Run with `npm test`.
//
// Uploads are unencrypted unless the sender turns on password protection, and
// the server can read those. The link preview for a drop used to call every
// file "Encrypted" and every folder "this encrypted vault", and the Terms,
// landing, Home and signup copy said the server never sees plaintext. These
// check the preview against the file's own flag and keep those claims out.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import * as nodeModule from 'node:module';

const lib = new URL('../src/lib/', import.meta.url).href;
// fetch_resource answers whatever the test put in globalThis.fetchResourceReply;
// a reply of null makes the call fail, as a 404 for a locked link does.
const network =
	'data:text/javascript,' +
	encodeURIComponent(`export const ApiServerRoutes = { fetchResource: '/fetch_resource' };
export const ApiServerClient = {
	post: async () => {
		const reply = globalThis.fetchResourceReply;
		if (reply === null) throw Object.assign(new Error('404'), { response: { status: 404 } });
		return { data: reply };
	}
};`);
const hooks = `
const lib = ${JSON.stringify(lib)};
const network = ${JSON.stringify(network)};
export function resolve(specifier, context, next) {
	if (specifier === '$lib/network.js') return { url: network, shortCircuit: true };
	if (specifier.startsWith('$lib/')) return { url: lib + specifier.slice(5), shortCircuit: true };
	return next(specifier, context);
}`;
nodeModule.register('data:text/javascript,' + encodeURIComponent(hooks));

const { load } = await import('../src/routes/[slug]/+page.server.js');
const DROP = '7f3c2a10-5b9e-4d1c-9a8e-2f6b1c0d4e5a';

/** The preview text for a drop whose fetch_resource reply is `data`. */
async function preview(/** @type {any} */ data) {
	/** @type {any} */ (globalThis).fetchResourceReply = data === null ? null : { data };
	const original = console.error;
	console.error = () => {};
	try {
		const { og } = /** @type {any} */ (await load(/** @type {any} */ ({ params: { slug: DROP } })));
		return `${og.title} ${og.description}`;
	} finally {
		console.error = original;
	}
}

const file = (/** @type {boolean} */ encrypted) => ({
	id: DROP,
	name: 'report.pdf',
	size: 2048,
	encrypted
});

test('the preview of an unencrypted file does not call it encrypted', async () => {
	const text = await preview({ resource_type: 'file', file: file(false) });
	assert.match(text, /report\.pdf/);
	assert.doesNotMatch(text, /encrypt/i);
});

test('the preview of an encrypted file says so', async () => {
	const text = await preview({ resource_type: 'file', file: file(true) });
	assert.match(text, /encrypted/i);
});

test('a folder is called encrypted only when every file in it is', async () => {
	const folder = { id: DROP, name: 'Q3' };
	const plain = await preview({
		resource_type: 'folder',
		folder,
		files: [file(false), file(false)]
	});
	assert.doesNotMatch(plain, /encrypt|vault/i);

	const mixed = await preview({
		resource_type: 'folder',
		folder,
		files: [file(true), file(false)]
	});
	assert.doesNotMatch(mixed, /encrypt|vault/i);

	const sealed = await preview({ resource_type: 'folder', folder, files: [file(true)] });
	assert.match(sealed, /encrypted/i);

	const empty = await preview({ resource_type: 'folder', folder, files: [] });
	assert.doesNotMatch(empty, /encrypt|vault/i);
});

test('the fallback preview, which knows nothing about the file, claims no encryption', async () => {
	for (const reply of [null, undefined, { resource_type: 'unknown' }]) {
		const text = await preview(reply);
		assert.doesNotMatch(text, /encrypt|zero-knowledge/i, String(reply));
	}
});

const page = (/** @type {string} */ path) =>
	readFileSync(new URL(`../src/routes/${path}`, import.meta.url), 'utf8').replace(/\s+/g, ' ');

test('the Terms do not say the server never sees plaintext', () => {
	const terms = page('policies/terms-of-service/+page.svelte');
	assert.doesNotMatch(terms, /never see your plaintext/i);
	assert.doesNotMatch(terms, /all encryption happens client-side/i);
	assert.doesNotMatch(terms, /provides end-to-end encrypted file storage/i);
	assert.match(terms, /unencrypted/i);
});

test('the disclaimer and acceptable use policy do not say every file is encrypted', () => {
	const disclaimer = page('policies/disclaimer/+page.svelte');
	assert.doesNotMatch(disclaimer, /we never hold your keys and cannot see your files/i);
	assert.match(disclaimer, /unencrypted/i);
	const aup = page('policies/acceptable-use/+page.svelte');
	assert.doesNotMatch(aup, /<p> SiloCat is a zero-knowledge, end-to-end encrypted/i);
});

test('landing, Home and signup do not promise that every upload is encrypted', () => {
	assert.doesNotMatch(page('+page.svelte'), /ciphertext and nothing else/i);
	assert.doesNotMatch(page('home/+page.svelte'), /encrypted end to end/i);
	assert.doesNotMatch(page('auth/signup/+page.svelte'), /encrypted end to end/i);
});
