// What silo.cat says about encryption. Run with `npm test`.
//
// Uploads are unencrypted unless the sender turns on password protection, and
// the server can read those. The link preview for a drop used to call every
// file "Encrypted" and every folder "this encrypted vault", and the Terms,
// landing, Home, signup, DMCA, privacy, API and plan copy said the server never
// sees plaintext. These check the preview and the drop page footer against the
// files' own flags and keep those claims out of the copy.
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

/** Fails with a short message, not the whole page, when `re` is in `src`. */
function absent(/** @type {string} */ src, /** @type {RegExp} */ re, /** @type {string} */ name) {
	const hit = src.match(re);
	assert.equal(hit, null, `${name} still says "${hit?.[0]}"`);
}

test('the drop page footer calls a folder encrypted only when every file in it is', async () => {
	const drop = page('[slug]/+page.svelte');
	const footer = drop.slice(drop.indexOf('class="secure-note"'));
	const claim = footer.slice(0, footer.indexOf('Encrypted end to end'));
	assert.ok(claim.length > 0, 'footer not found');
	absent(claim, /folderFiles\.some/, 'claim');
	assert.ok(/allEncrypted\(folderFiles\)/.test(claim), 'footer does not use allEncrypted');

	const { allEncrypted } = await import('../src/lib/encryption.js');
	assert.equal(allEncrypted([file(true), file(true)]), true);
	assert.equal(allEncrypted([file(true), file(false)]), false);
	assert.equal(allEncrypted([file(false)]), false);
	assert.equal(allEncrypted([]), false);
	assert.equal(allEncrypted(undefined), false);
});

test('the DMCA and privacy policies do not say every file is ciphertext', () => {
	const dmca = page('policies/dmca-policy/+page.svelte');
	absent(dmca, /architecture prevents us from viewing user content/i, 'dmca');
	absent(dmca, /encrypted blob/i, 'dmca');
	assert.ok(/unencrypted/i.test(dmca), 'DMCA policy never mentions unencrypted files');
	const privacy = page('policies/privacy-policy/+page.svelte');
	absent(privacy, /ciphertext plus metadata, and nothing more/i, 'privacy');
	absent(privacy, /for encrypted storage/i, 'privacy');
});

test('the API docs do not say the API only accepts ciphertext', () => {
	const api = page('api/+page.svelte');
	absent(api, /the API cannot accept a file/i, 'api');
	absent(api, /the API only ever sees ciphertext/i, 'api');
	absent(api, /ciphertext is all that ever leaves your process/i, 'api');
	absent(api, /Ciphertext is scrubbed/i, 'api');
});

test('plan features do not call the storage itself encrypted', () => {
	for (const path of ['pricing/+page.svelte', 'home/billing/+page.svelte']) {
		const src = page(path);
		absent(src, /\d+ [GT]B encrypted storage/i, path);
		absent(src, /'End-to-end encryption'/, path);
	}
});

test('page descriptions that mention encryption say it is optional', () => {
	const sources = [
		'../lib/seo.js',
		'+page.svelte',
		'about/+page.svelte',
		'pricing/+page.svelte',
		'api/+page.svelte',
		'policies/terms-of-service/+page.svelte',
		'policies/acceptable-use/+page.svelte',
		'policies/disclaimer/+page.svelte',
		'policies/dmca-policy/+page.svelte',
		'policies/privacy-policy/+page.svelte'
	];
	for (const path of sources) {
		const src = page(path);
		const descriptions = [
			...src.matchAll(/description="([^"]*)"/g),
			...src.matchAll(/defaultDescription: '([^']*)'/g)
		].map((m) => m[1]);
		assert.ok(descriptions.length > 0, `no description in ${path}`);
		for (const d of descriptions) {
			if (/encrypt|zero-knowledge|ciphertext/i.test(d)) {
				assert.match(d, /optional|password|encryption is off/i, `${path}: ${d}`);
			}
		}
	}
});

const repo = (/** @type {string} */ path) =>
	readFileSync(new URL(`../../../${path}`, import.meta.url), 'utf8').replace(/\s+/g, ' ');

test('the empty Files view does not say an upload is encrypted', () => {
	const files = page('home/files/+page.svelte');
	absent(files, /It is encrypted before it leaves your browser/i, 'home/files');
	absent(page('home/+page.svelte'), /It is encrypted before it leaves your browser/i, 'home');
});

test('the sign-in page description does not call the storage encrypted', () => {
	const signin = page('auth/signin/+page.svelte');
	for (const [, d] of signin.matchAll(/description="([^"]*)"/g)) {
		if (/encrypt|zero-knowledge|ciphertext/i.test(d)) {
			assert.match(d, /optional|password/i, `auth/signin: ${d}`);
		}
	}
});

test('the security pipeline does not say every upload is ciphertext', () => {
	const privacy = page('privacy/+page.svelte');
	absent(privacy, /what: 'Only ciphertext travels/i, 'privacy');
	absent(privacy, /what: "The recipient's browser fetches ciphertext/i, 'privacy');
	const readme = repo('README.md');
	absent(readme, /\| Upload \| Only ciphertext travels/i, 'README.md');
	absent(readme, /\| Download \| The recipient's browser fetches ciphertext/i, 'README.md');
});

test('the drop page fallback preview matches the server one', () => {
	const drop = page('[slug]/+page.svelte');
	absent(drop, /Secure download on Silocat/i, '[slug]');
	absent(drop, /A file shared securely/i, '[slug]');
	const server = page('[slug]/+page.server.js');
	const title = server.match(/title: '(Download on SiloCat)'/)?.[1];
	assert.ok(title, 'server fallback title not found');
	assert.ok(drop.includes(`title: '${title}'`), 'client fallback title differs from the server one');
});

test('the web manifest does not call every file encrypted', () => {
	const manifest = JSON.parse(readFileSync(new URL('../static/site.webmanifest', import.meta.url), 'utf8'));
	absent(manifest.name, /encrypt|zero-knowledge/i, 'manifest name');
	if (/encrypt|zero-knowledge/i.test(manifest.description)) {
		assert.match(manifest.description, /optional/i, `manifest description: ${manifest.description}`);
	}
	absent(manifest.description, /^Zero-knowledge, end-to-end encrypted/i, 'manifest description');
});

test('the npm client docs do not say it only ever sends ciphertext', () => {
	const readme = repo('packages/client/README.md');
	absent(readme, /Silocat is zero-knowledge\. Files are encrypted before upload/i, 'client README');
	absent(readme, /it takes ciphertext and hands back/i, 'client README');
	absent(readme, /so ciphertext is all that ever leaves your process/i, 'client README');
	const index = repo('packages/client/src/index.js');
	absent(index, /the server holds ciphertext and never sees/i, 'client index.js');
	absent(index, /Encrypt and upload\. The plaintext never leaves this process/i, 'client index.js');
	const pkg = JSON.parse(readFileSync(new URL('../../../packages/client/package.json', import.meta.url), 'utf8'));
	absent(pkg.description, /ciphertext is all that ever leaves/i, 'client package.json');
});

test('the email templates do not say every file is encrypted', () => {
	const sources = [
		'email_templates/build.py',
		...['account-suspended', 'password-reset', 'ticket-update', 'verify-email'].map(
			(t) => `email_templates/transactional/${t}.html`
		)
	];
	for (const path of sources) {
		const src = repo(path);
		absent(src, /&middot;&nbsp; end-to-end encrypted file sharing/i, path);
		absent(src, /Your files stay encrypted throughout/i, path);
		absent(src, /We hold ciphertext and never see/i, path);
		absent(src, /remain encrypted exactly as you left them/i, path);
	}
});

test('the security pipeline says who can read an unencrypted file at every stage', () => {
	// Every stage from Upload to Download reads differently for a file sent
	// without password protection, so the "who" has to say so too.
	const later = ['Upload', 'Store', 'Share', 'Download'];
	const privacy = page('privacy/+page.svelte');
	const rows = [
		...privacy.matchAll(
			/stage: '(\w+)',.*?what: (['"])((?:\\.|(?!\2).)*)\2, who: '([^']*)', tone: '(\w+)'/g
		)
	].map(([, stage, , what, who, tone]) => ({ stage, what, who, tone }));
	assert.deepEqual(
		rows.map((r) => r.stage),
		['Select', 'Encrypt', 'Upload', 'Store', 'Share', 'Download', 'Delete'],
		'privacy stages not parsed'
	);
	for (const { stage, what, who, tone } of rows.filter((r) => later.includes(r.stage))) {
		assert.match(what, /unencrypted|without password protection|plain file/i, `privacy ${stage} what: ${what}`);
		assert.match(who, /unencrypted|otherwise|if encrypted/i, `privacy ${stage} who: ${who}`);
		assert.notEqual(tone, 'ok', `privacy ${stage} tone`);
	}
	const readme = repo('README.md');
	const table = [...readme.matchAll(/\| (\w+) \| ([^|]*) \| ([^|]*) \|/g)].filter(([, s]) =>
		later.includes(s)
	);
	assert.equal(table.length, later.length, 'README pipeline rows not parsed');
	for (const [, stage, what, who] of table) {
		assert.match(what, /unencrypted|without password protection|plain file/i, `README ${stage} what: ${what}`);
		assert.match(who, /unencrypted|otherwise|if encrypted/i, `README ${stage} who: ${who}`);
	}
});

test('the README and security policy do not say the server only ever holds ciphertext', () => {
	const readme = repo('README.md');
	absent(readme, /so ciphertext is all the store ever sees/i, 'README.md');
	absent(readme, /the HTTP API takes ciphertext rather than files/i, 'README.md');
	absent(readme, /what we guarantee: we hold ciphertext/i, 'README.md');
	const security = repo('SECURITY.md');
	absent(security, /what we don't We hold ciphertext\./i, 'SECURITY.md');
	absent(security, /limited by the fact that we cannot read file contents/i, 'SECURITY.md');
	const guarantee = security.slice(security.indexOf("## What we guarantee"), security.indexOf('## Self-hosting'));
	assert.ok(guarantee.length > 30, 'SECURITY.md guarantee section not found');
	assert.match(guarantee, /without password protection|unencrypted/i, 'SECURITY.md guarantee section');
	const short = readme.slice(readme.indexOf('The short version of what we guarantee'), readme.indexOf('## Licence'));
	assert.match(short, /without password protection|unencrypted/i, 'README.md security summary');
});

test('the empty-state hints name the encryption control the upload form shows', () => {
	const files = page('home/files/+page.svelte');
	assert.ok(files.includes('<span class="t1">Client-side encryption</span>'), 'upload form label changed');
	const hints = [
		['home/files', files.match(/class="state-line">(.*?)<\/span>/)?.[1]],
		['home', page('home/+page.svelte').match(/class="empty-line">(.*?)<\/span>/)?.[1]]
	];
	for (const [name, hint] of hints) {
		assert.ok(hint, `${name} empty hint not found`);
		absent(hint, /password protection/i, name);
		assert.match(hint, /client-side encryption/i, `${name}: ${hint}`);
	}
});

test('page titles do not call the whole product encrypted', () => {
	const titles = [
		['lib/seo.js', repo('services/web_server/src/lib/seo.js').match(/defaultTitle: '([^']*)'/)?.[1]],
		['landing', page('+page.svelte').match(/<Seo title="([^"]*)"/)?.[1]],
		['about', page('about/+page.svelte').match(/<Seo title="([^"]*)"/)?.[1]]
	];
	for (const [name, title] of titles) {
		assert.ok(title, `${name} title not found`);
		if (/encrypt|zero-knowledge/i.test(title)) {
			assert.match(title, /optional/i, `${name} title: ${title}`);
		}
	}
});
