// The notice an account sees when it holds as much as its plan allows or more.
// Run with `npm test`.
//
// A lapsed plan leaves an account over its base storage. It keeps every file,
// read-only, and the server refuses uploads and restores until usage is under
// the plan. The files page used to show nothing about it, so every upload
// just failed.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { isOverQuota, OVER_QUOTA_NOTICE } from '../src/lib/storageQuota.js';

const read = (path) => readFileSync(new URL(path, import.meta.url), 'utf8');

test('the server flag decides when it is present', () => {
	assert.equal(isOverQuota({ total: 100, used: 500, free: 0, over_quota: true }), true);
	assert.equal(isOverQuota({ total: 1000, used: 400, free: 600, over_quota: false }), false);
});

test('a server without the flag is read from used and total', () => {
	assert.equal(isOverQuota({ total: 100, used: 500, free: -400 }), true);
	assert.equal(isOverQuota({ total: 300, used: 300, free: 0 }), true);
	assert.equal(isOverQuota({ total: 1000, used: 400, free: 600 }), false);
});

test('stats that have not loaded or failed show no notice', () => {
	assert.equal(isOverQuota(undefined), false);
	assert.equal(isOverQuota(null), false);
	assert.equal(isOverQuota({ total: 0, used: 0, free: 0 }), false);
});

test('the notice says files stay and when uploads come back', () => {
	assert.match(OVER_QUOTA_NOTICE, /readable and shareable/);
	assert.match(OVER_QUOTA_NOTICE, /under your plan/);
	assert.doesNotMatch(OVER_QUOTA_NOTICE, /\u2014/);
});

test('the files page, its upload form and the trash show the notice', () => {
	const files = read('../src/routes/home/files/+page.svelte');
	assert.equal((files.match(/<OverQuotaNotice\b/g) || []).length, 2, 'files page and upload form');
	const upload = files.slice(files.indexOf('title="Upload files"'));
	assert.match(upload, /<OverQuotaNotice\b/);
	assert.match(read('../src/routes/home/trash/+page.svelte'), /<OverQuotaNotice\b/);
});
