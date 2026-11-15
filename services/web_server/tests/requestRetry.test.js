// A failed upload through a request link must give its slot back. Run with
// `npm test`.
//
// Each upload takes one of the link's slots when it starts. The page has no
// per-chunk retry: when a PUT or mark-complete fails it shows the error, and
// trying again starts a new upload. On a link capped at one, that retry found
// the link full, and stayed refused until the server reaped the dead upload
// hours later.
import { test, beforeEach } from 'node:test';
import assert from 'node:assert/strict';
import './stubs/hooks.js';

const axios = (await import('axios')).default;
const { uploadToRequest } = await import('../src/lib/requestUpload.js');

/** Every POST the upload made, as [path, body]. */
let posts = [];
/** Paths whose next call fails. */
let failing = new Set();

beforeEach(() => {
	posts = [];
	failing = new Set();
	axios.post = /** @type {any} */ (
		async (/** @type {string} */ path, /** @type {any} */ body) => {
			posts.push([path, body]);
			if (failing.has(path)) throw new Error('offline: ' + path);
			if (path.endsWith('/upload')) {
				return {
					data: {
						success: {
							data: {
								file: { id: 'file-1' },
								chunks: [{ id: 'chunk-1', chunk_index: 0, presigned_url: 'https://storage/put' }]
							}
						}
					}
				};
			}
			return { data: { success: { data: {} } } };
		}
	);
	axios.put = /** @type {any} */ (
		async (/** @type {string} */ url) => {
			if (failing.has(url)) throw new Error('offline: ' + url);
			return { status: 200 };
		}
	);
});

const file = () => new File(['hello'], 'notes.txt', { type: 'text/plain' });
const gaveUp = () => posts.filter(([path]) => path === '/api/v1/public/request/abandon');

test('a chunk that fails to upload gives the slot back, then reports the failure', async () => {
	failing.add('https://storage/put');
	await assert.rejects(uploadToRequest('tok', file()), /offline: https:\/\/storage\/put/);
	assert.deepEqual(gaveUp(), [['/api/v1/public/request/abandon', { token: 'tok', file_id: 'file-1' }]]);
});

test('a chunk that fails to mark complete gives the slot back too', async () => {
	failing.add('/api/v1/public/request/mark-complete');
	await assert.rejects(uploadToRequest('tok', file()), /offline: \/api\/v1\/public\/request\/mark-complete/);
	assert.equal(gaveUp().length, 1);
});

test('when giving the slot back fails as well, the upload error is the one shown', async () => {
	failing.add('https://storage/put');
	failing.add('/api/v1/public/request/abandon');
	await assert.rejects(uploadToRequest('tok', file()), /offline: https:\/\/storage\/put/);
	assert.equal(gaveUp().length, 1);
});

test('a finished upload keeps its slot', async () => {
	const sent = await uploadToRequest('tok', file());
	assert.deepEqual(sent, { id: 'file-1' });
	assert.equal(gaveUp().length, 0);
});

test('an upload the server refused has no slot to give back', async () => {
	failing.add('/api/v1/public/request/upload');
	await assert.rejects(uploadToRequest('tok', file()));
	assert.equal(gaveUp().length, 0);
});
