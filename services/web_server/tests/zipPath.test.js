// Entry names in the zips the browser builds for folder downloads. Run with
// `npm test`.
//
// File and folder names come from whoever uploaded them: an anonymous drop, a
// share owner, or an outsider uploading through a request link. JSZip writes
// an entry name exactly as given, so a file called "../../Startup/update.bat"
// or a folder called ".." made a zip that extracts outside the folder the
// recipient chose. The API refuses such names now, but files stored before it
// did still exist, so the zip side has to hold on its own.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync, readdirSync, statSync } from 'node:fs';
import { join } from 'node:path';
import JSZip from 'jszip';
import { zipEntryPath } from '../src/lib/zipPath.js';

/** The entry names of a zip built the way the download pages build one. */
async function entriesFor(items) {
	const zip = new JSZip();
	for (const { path, name } of items) zip.file(zipEntryPath(path, name), 'x');
	const loaded = await JSZip.loadAsync(await zip.generateAsync({ type: 'uint8array' }));
	// Read the names as written, not as loadAsync cleans them up.
	return Object.values(loaded.files).map((f) => f.unsafeOriginalName ?? f.name);
}

function escapes(entry) {
	const parts = entry.split(/[\\/]/);
	return (
		entry.startsWith('/') ||
		entry.startsWith('\\') ||
		// A drive prefix in any segment: "C:x" is drive relative, and an extractor
		// that joins segment by segment restarts at the drive.
		parts.some((p) => /^[a-zA-Z][:\uff1a]/.test(p)) ||
		parts.some((p) => /^[.\s]+$/.test(p) && p.includes('..')) ||
		parts.some((p) => p.trim() === '.') ||
		// eslint-disable-next-line no-control-regex
		/[\u0000-\u001f\u007f-\u009f]/.test(entry)
	);
}

test('a stored name or folder path cannot climb out of the extract folder', async () => {
	const names = await entriesFor([
		{
			path: '',
			name: '../../../AppData/Roaming/Microsoft/Windows/Start Menu/Programs/Startup/update.bat'
		},
		{ path: '', name: '..\\..\\evil.bat' },
		{ path: '..', name: 'a.txt' },
		{ path: '../..', name: 'b.txt' },
		{ path: 'a/../..', name: 'c.txt' },
		{ path: 'ok/...', name: 'd.txt' },
		{ path: 'ok/.. ', name: 'e.txt' },
		{ path: '/etc', name: 'passwd' },
		{ path: '', name: '/etc/passwd' },
		{ path: 'x', name: '.' },
		{ path: '', name: 'line\nbreak\u0000.txt' },
		{ path: 'C:', name: 'x.bat' },
		{
			path: 'C:/ProgramData/Microsoft/Windows/Start Menu/Programs/StartUp',
			name: 'x.bat'
		},
		{ path: '', name: 'C:evil.bat' },
		{ path: 'ok', name: 'd:x.bat' },
		{ path: '', name: 'C\uff1aevil.bat' }
	]);
	const bad = names.filter(escapes);
	assert.deepEqual(bad, []);
});

test('ordinary names and folder structure come through unchanged', async () => {
	assert.equal(zipEntryPath('', 'notes.txt'), 'notes.txt');
	assert.equal(zipEntryPath(null, 'notes.txt'), 'notes.txt');
	assert.equal(zipEntryPath(undefined, '.env'), '.env');
	assert.equal(zipEntryPath('Photos/2026 trip', 'IMG 001.jpg'), 'Photos/2026 trip/IMG 001.jpg');
	assert.equal(zipEntryPath('a..b', '..hidden'), 'a..b/..hidden');
	assert.equal(zipEntryPath('Docs', 'Report: Q3.pdf'), 'Docs/Report: Q3.pdf');
	assert.equal(zipEntryPath('名前', 'é.png'), '名前/é.png');
});

test('fullwidth look-alikes of / \\ and . cannot stand in for them', async () => {
	// Windows code pages map these to their ASCII forms when a tool converts the
	// name to the ANSI API, so "\uff0e\uff0e\uff0f" reads as "../" there.
	const names = await entriesFor([
		{ path: '', name: '\uff0e\uff0e\uff0f\uff0e\uff0e\uff0fx.bat' },
		{ path: '\uff0e\uff0e', name: 'a.txt' },
		{ path: 'ok/\uff0e\uff0e\uff3c\uff0e\uff0e', name: 'b.txt' },
		{ path: '', name: 'a\u2215b\u2216c.txt' }
	]);
	const ascii = names.map((n) =>
		n
			.replace(/[\uff0f\u2215]/g, '/')
			.replace(/[\uff3c\u2216]/g, '\\')
			.replace(/\uff0e/g, '.')
	);
	assert.deepEqual(ascii.filter(escapes), []);
	assert.deepEqual(
		names.filter((n) => /[\uff0f\uff3c\u2215\u2216]/.test(n)),
		[]
	);
});

test('a hostile name stays one entry in its own folder', () => {
	assert.equal(zipEntryPath('Photos', '../../x.bat'), 'Photos/.._.._x.bat');
	assert.equal(zipEntryPath('..', 'a.txt'), '_/a.txt');
	assert.equal(zipEntryPath('', '..'), '_');
	assert.equal(zipEntryPath('C:', 'x.bat'), 'C_/x.bat');
	assert.equal(zipEntryPath('', 'C:evil.bat'), 'C_evil.bat');
	assert.equal(zipEntryPath('Docs', 'Report: Q3.pdf'), 'Docs/Report: Q3.pdf');
});

test('every zip the web app builds names its entries through zipEntryPath', () => {
	const root = new URL('../src/', import.meta.url).pathname;
	const files = [];
	(function walk(dir) {
		for (const entry of readdirSync(dir)) {
			const p = join(dir, entry);
			if (statSync(p).isDirectory()) walk(p);
			else if (/\.(svelte|js|ts)$/.test(p)) files.push(p);
		}
	})(root);
	const raw = [];
	for (const file of files) {
		const src = readFileSync(file, 'utf8');
		for (const m of src.matchAll(/\bzip\.file\(([^,]*),/g)) {
			if (!m[1].trim().startsWith('zipEntryPath(')) raw.push(`${file}: zip.file(${m[1]}, ...)`);
		}
	}
	assert.deepEqual(raw, []);
});
