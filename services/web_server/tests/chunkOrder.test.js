// Encrypted chunks have to come back in the order, and the number, they were
// uploaded in. Run with `npm test`.
//
// Each chunk is sealed on its own, so a server that reorders the chunk list,
// drops its tail or repeats a row used to get a "successful" download of a file
// that was not the one uploaded. These drive the real download code (lib/
// download.js and the npm client in packages/client) against a fake server that
// does exactly that, and check every upload path writes the chunk positions the
// downloads rely on.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync, readdirSync, statSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import './stubs/hooks.js';

const sodium = (await import('libsodium-wrappers-sumo')).default;
await sodium.ready;
const axios = (await import('axios')).default;
const { get } = await import('svelte/store');
const { framedNonce, assertChunkOrder, chunkOrderCheck, FRAME_TAG } = await import(
	'../src/lib/chunkOrder.js'
);
const webCrypto = await import('./stubs/cryptoClient.js');
const { downloadFile, fetchDecryptedBlob, downloads } = await import('../src/lib/download.js');
const { Silocat } = await import('../../../packages/client/src/index.js');
const clientCrypto = await import('../../../packages/client/src/crypto.js');

const PARTS = ['ROW1: pay 100 to alice\n', 'ROW2: pay 999 to mallory\n', 'ROW3: total 1099\n'];
const PASSWORD = 'correct-horse-battery-staple';
const b64 = (/** @type {Uint8Array} */ bytes) => Buffer.from(bytes).toString('base64');
const fromB64 = (/** @type {string} */ s) => new Uint8Array(Buffer.from(s, 'base64'));

/**
 * A stored file: chunk rows as the server returns them, and the R2 objects.
 * `framed` false gives the random nonces every upload used before this change.
 */
function seal(/** @type {Uint8Array} */ key, /** @type {Uint8Array} */ salt, framed = true) {
	const seed = sodium.randombytes_buf(24);
	const objects = new Map();
	const rows = PARTS.map((text, i) => {
		const nonce = framed ? framedNonce(seed, i, PARTS.length) : sodium.randombytes_buf(24);
		const url = `https://r2.test/obj/${b64(sodium.randombytes_buf(9))}`;
		const plain = sodium.from_string(text);
		objects.set(url, sodium.crypto_aead_xchacha20poly1305_ietf_encrypt(plain, null, null, nonce, key));
		return { id: `c${i}`, size: plain.length, salt: b64(salt), nonce: b64(nonce), presigned_url: url };
	});
	return { rows, objects };
}

/** What a server with write access to the chunks table can hand back. */
const TAMPER = {
	reordered: (/** @type {any[]} */ r) => [r[1], r[0], r[2]],
	truncated: (/** @type {any[]} */ r) => [r[0], r[1]],
	duplicated: (/** @type {any[]} */ r) => [r[0], r[1], r[1], r[2]]
};

// ---- the web app's download paths ------------------------------------------

const webSalt = sodium.randombytes_buf(16);
const webKey = await webCrypto.deriveKey(PASSWORD, webSalt);

/** Points axios at a fake server that serves `rows` for every fetch-chunks call. */
function serveWeb(/** @type {any[]} */ rows, /** @type {Map<string, Uint8Array>} */ objects) {
	axios.post = /** @type {any} */ (async () => ({ data: { data: { chunks: rows } } }));
	axios.get = /** @type {any} */ (
		async (/** @type {string} */ url) => {
			const body = objects.get(url);
			if (!body) throw new Error(`no object at ${url}`);
			return { data: body.slice().buffer };
		}
	);
}

const webFile = { id: 'f1', name: 'ledger.txt', mime: 'text/plain', size: 64, encrypted: true };

test('the preview path returns an intact encrypted file', async () => {
	const { rows, objects } = seal(webKey, webSalt);
	serveWeb(rows, objects);
	const blob = await fetchDecryptedBlob(webFile, { password: PASSWORD });
	assert.equal(await blob.text(), PARTS.join(''));
});

test('the preview path still reads files uploaded with random nonces', async () => {
	const { rows, objects } = seal(webKey, webSalt, false);
	serveWeb(rows, objects);
	const blob = await fetchDecryptedBlob(webFile, { password: PASSWORD });
	assert.equal(await blob.text(), PARTS.join(''));
});

for (const [how, tamper] of Object.entries(TAMPER)) {
	test(`the preview path refuses a ${how} chunk list`, async () => {
		const { rows, objects } = seal(webKey, webSalt);
		serveWeb(tamper(rows), objects);
		await assert.rejects(fetchDecryptedBlob(webFile, { password: PASSWORD }), /out of order/);
	});

	test(`a saved download of a ${how} chunk list fails and is thrown away`, async () => {
		const { rows, objects } = seal(webKey, webSalt);
		serveWeb(tamper(rows), objects);
		const name = `ledger-${how}.txt`;
		await downloadFile({ ...webFile, name }, { password: PASSWORD });
		const entry = get(downloads).find((d) => d.name === name);
		assert.equal(entry?.status, 'error', JSON.stringify(entry));
		const sink = /** @type {any} */ (globalThis).lastSink;
		assert.equal(sink.closed, false);
		assert.equal(sink.writes.length, 0, 'no chunk of a tampered file reaches the disk');
	});
}

test('a saved download of an intact file completes', async () => {
	const { rows, objects } = seal(webKey, webSalt);
	serveWeb(rows, objects);
	await downloadFile({ ...webFile, name: 'ledger-ok.txt' }, { password: PASSWORD });
	const entry = get(downloads).find((d) => d.name === 'ledger-ok.txt');
	assert.equal(entry?.status, 'done', JSON.stringify(entry));
	const sink = /** @type {any} */ (globalThis).lastSink;
	assert.equal(Buffer.concat(sink.writes).toString(), PARTS.join(''));
});

// ---- the npm client --------------------------------------------------------

const clientSalt = await clientCrypto.randomSalt();
const clientKey = await clientCrypto.deriveKey(PASSWORD, clientSalt);

/**
 * A fake Silocat API for the npm client. Uploads land in `stored`; fetch-chunks
 * returns `serve(stored.rows)`, which is where a test does its tampering.
 */
function fakeServer(serve = (/** @type {any[]} */ rows) => rows) {
	const stored = { rows: /** @type {any[]} */ ([]), objects: new Map() };
	const json = (/** @type {any} */ body) => new Response(JSON.stringify(body), { status: 200 });
	/** @type {typeof fetch} */
	const fetch = async (input, init = {}) => {
		const url = String(input);
		const method = init.method ?? 'GET';
		if (url.startsWith('https://r2.test/')) {
			if (method === 'PUT') {
				stored.objects.set(url, new Uint8Array(/** @type {any} */ (init.body)));
				return new Response(null, { status: 200 });
			}
			const body = stored.objects.get(url);
			return body ? new Response(body.slice()) : new Response(null, { status: 404 });
		}
		const path = new URL(url).pathname;
		const body = init.body ? JSON.parse(String(init.body)) : {};
		if (path === '/api/v1/sanctum/file') {
			stored.rows = body.chunks.map((/** @type {any} */ c, /** @type {number} */ i) => ({
				id: `c${i}`,
				size: c.size,
				salt: c.salt,
				nonce: c.nonce,
				presigned_url: `https://r2.test/obj/up${i}`
			}));
			return json({ data: { file: { id: 'f1' }, chunks: stored.rows } });
		}
		if (path === '/api/v1/sanctum/file/mark-chunk-complete') return json({});
		if (path === '/api/v1/sanctum/file/fetch-chunks') return json({ data: { chunks: serve(stored.rows) } });
		return new Response('{}', { status: 404 });
	};
	return { stored, silo: new Silocat({ apiKey: 'sk_test', baseUrl: 'https://api.test', fetch }) };
}

test('the npm client round-trips its own encrypted upload', async () => {
	const { silo } = fakeServer();
	await silo.upload(new TextEncoder().encode('one chunk of text'), { name: 'a.txt', password: PASSWORD });
	const out = await silo.download('f1', { password: PASSWORD });
	assert.equal(Buffer.from(out).toString(), 'one chunk of text');
});

test('the npm client writes the chunk position into the nonce it uploads', async () => {
	const { silo, stored } = fakeServer();
	await silo.upload(new TextEncoder().encode('x'), { name: 'a.txt', password: PASSWORD });
	const nonce = fromB64(stored.rows[0].nonce);
	assert.equal(nonce.length, 24);
	assert.deepEqual([...nonce.subarray(8, 16)], [...FRAME_TAG]);
	assert.deepEqual([...nonce.subarray(16)], [0, 0, 0, 0, 0, 0, 0, 1]);
});

test('the npm client refuses its own upload served twice', async () => {
	const { silo } = fakeServer((rows) => [rows[0], rows[0]]);
	await silo.upload(new TextEncoder().encode('once'), { name: 'a.txt', password: PASSWORD });
	await assert.rejects(silo.download('f1', { password: PASSWORD }), /out of order/);
});

for (const [how, tamper] of Object.entries(TAMPER)) {
	test(`the npm client refuses a ${how} chunk list`, async () => {
		const { silo, stored } = fakeServer(tamper);
		const sealed = seal(clientKey, clientSalt);
		stored.rows = sealed.rows;
		stored.objects = sealed.objects;
		await assert.rejects(silo.download('f1', { password: PASSWORD }), /out of order/);
	});
}

test('the npm client still reads files uploaded with random nonces', async () => {
	const { silo, stored } = fakeServer();
	const sealed = seal(clientKey, clientSalt, false);
	stored.rows = sealed.rows;
	stored.objects = sealed.objects;
	const out = await silo.download('f1', { password: PASSWORD });
	assert.equal(Buffer.from(out).toString(), PARTS.join(''));
});

test('the npm client does not hand back plaintext the server swapped in for an encrypted file', async () => {
	// The server drops the nonces and points the chunk at bytes of its own. The
	// caller passed a password, so it expects ciphertext.
	const { silo, stored } = fakeServer();
	stored.rows = [{ id: 'c0', size: 9, salt: null, nonce: null, presigned_url: 'https://r2.test/obj/evil' }];
	stored.objects.set('https://r2.test/obj/evil', new TextEncoder().encode('attacker chosen content'));
	await assert.rejects(silo.download('f1', { password: PASSWORD }), /not encrypted|no nonce/);
});

test('the npm client still downloads an unencrypted file when no password is given', async () => {
	const { silo } = fakeServer();
	await silo.upload(new TextEncoder().encode('plain'), { name: 'p.txt' });
	const out = await silo.download('f1');
	assert.equal(Buffer.from(out).toString(), 'plain');
});

// ---- the check itself ------------------------------------------------------

const rowsFor = (/** @type {Uint8Array[]} */ nonces) => nonces.map((n) => ({ nonce: b64(n) }));

test('framed nonces carry the index and count, and nothing else checks out', () => {
	const seed = sodium.randombytes_buf(8);
	const nonces = [0, 1, 2].map((i) => framedNonce(seed, i, 3));
	assertChunkOrder(rowsFor(nonces));
	assert.throws(() => assertChunkOrder(rowsFor([nonces[0], nonces[2]])), /out of order/);
	// A chunk from another upload of the same length does not fit in.
	const other = framedNonce(sodium.randombytes_buf(8), 1, 3);
	assert.throws(() => assertChunkOrder(rowsFor([nonces[0], other, nonces[2]])), /out of order/);
	// Once a file is framed, a chunk with a random nonce cannot belong to it.
	assert.throws(
		() => assertChunkOrder(rowsFor([nonces[0], sodium.randombytes_buf(24), nonces[2]])),
		/out of order/
	);
	assert.throws(() => framedNonce(seed, 3, 3));
	assert.throws(() => framedNonce(seed, 0, 0));
});

test('the incremental check waits for the last chunk before it passes', () => {
	const seed = sodium.randombytes_buf(8);
	const check = chunkOrderCheck();
	check.next({ nonce: b64(framedNonce(seed, 0, 2)) });
	assert.throws(() => check.done(), /out of order/);
	check.next({ nonce: b64(framedNonce(seed, 1, 2)) });
	check.done();
});

// ---- every upload and download path uses it -------------------------------

const webRoot = fileURLToPath(new URL('../src/', import.meta.url));
const clientRoot = fileURLToPath(new URL('../../../packages/client/src/', import.meta.url));
/** The cipher's own definitions, which are not upload or download paths. */
const DEFINITIONS = new Set(['lib/chacha.js', 'lib/cryptoClient.js', 'lib/workers/crypto.worker.js', 'crypto.js']);

/** @returns {string[]} */
function sources(/** @type {string} */ root, dir = root) {
	return readdirSync(dir).flatMap((name) => {
		const path = join(dir, name);
		if (statSync(path).isDirectory()) return sources(root, path);
		return /\.(js|svelte)$/.test(name) ? [path.slice(root.length)] : [];
	});
}

const everySource = [
	...sources(webRoot).map((rel) => ({ rel, text: readFileSync(join(webRoot, rel), 'utf8') })),
	...sources(clientRoot).map((rel) => ({ rel, text: readFileSync(join(clientRoot, rel), 'utf8') }))
].filter(({ rel }) => !DEFINITIONS.has(rel));

test('every path that decrypts chunks checks their order first', () => {
	const decrypting = everySource.filter(({ text }) => /\bdecryptChunk\(/.test(text));
	assert.ok(decrypting.length >= 5, decrypting.map((f) => f.rel).join(', '));
	for (const { rel, text } of decrypting) {
		assert.match(text, /\bassertChunkOrder\(|\bchunkOrderCheck\(/, rel);
		if (/\bchunkOrderCheck\(/.test(text)) assert.match(text, /\.done\(\)/, rel);
	}
});

test('every path that encrypts chunks writes their position into the nonce', () => {
	const encrypting = everySource.filter(({ text }) => /\bencryptChunk\(/.test(text));
	assert.ok(encrypting.length >= 4, encrypting.map((f) => f.rel).join(', '));
	for (const { rel, text } of encrypting) {
		assert.match(text, /\bframedNonce\(/, rel);
	}
});

test('the npm client carries an identical copy of the check', () => {
	assert.equal(
		readFileSync(join(clientRoot, 'chunkOrder.js'), 'utf8'),
		readFileSync(join(webRoot, 'lib/chunkOrder.js'), 'utf8')
	);
});
