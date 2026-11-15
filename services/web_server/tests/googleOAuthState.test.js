// Google sign-in must carry a `state` that only the browser which started it
// holds. Without one, anyone can start a sign-in to their own Google account,
// keep the code, and send a victim /auth/callback?code=...: the victim's
// browser then trades it for a session in the attacker's account, and what the
// victim uploads lands in the attacker's drive. Run with `npm test`.
import { test, beforeEach } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import * as nodeModule from 'node:module';

const here = new URL('./stubs/', import.meta.url).href;
const lib = new URL('../src/lib/', import.meta.url).href;
const env =
	'data:text/javascript,' +
	encodeURIComponent(
		'export const env = { JWT_SECRET: "test-jwt-secret", OAUTH_ID_GOOGLE: "test-client.apps.googleusercontent.com" };'
	);
const appEnv = 'data:text/javascript,' + encodeURIComponent('export const dev = false;');
nodeModule.register(
	'data:text/javascript,' +
		encodeURIComponent(`
const here = ${JSON.stringify(here)};
const lib = ${JSON.stringify(lib)};
export function resolve(specifier, context, next) {
	if (specifier === '$env/dynamic/private') return { url: ${JSON.stringify(env)}, shortCircuit: true };
	if (specifier === '$app/environment') return { url: ${JSON.stringify(appEnv)}, shortCircuit: true };
	if (specifier === '$lib/network.js') return { url: here + 'network.js', shortCircuit: true };
	if (specifier.startsWith('$lib/')) return { url: lib + specifier.slice(5), shortCircuit: true };
	return next(specifier, context);
}`)
);

const { handle } = await import('../src/hooks.server.js');
const { SESSION_COOKIE } = await import('../src/lib/auth.js');
const { decodeToken } = await import('../src/lib/jwt.js');

const STATE_COOKIE = 'sc_oauth_state';
const attacker = { id: 'u-attacker', email: 'attacker@evil.example', api_key: 'attacker-key' };

/** Every call that reached api_switch, as [method, path, body]. */
let calls = [];

beforeEach(() => {
	calls = [];
	globalThis.fakeApi = {
		get: async () => {
			throw { response: { status: 401, data: { message: 'Unauthorized' } } };
		},
		post: async (path, body) => {
			calls.push(['post', path, body]);
			// Whoever holds the code gets the account it was issued for.
			return { status: 200, data: { status: 200, data: { ...attacker } } };
		}
	};
});

function jarOf(entries = {}) {
	const jar = new Map(Object.entries(entries));
	const set = [];
	return {
		jar,
		set,
		cookies: {
			get: (name) => jar.get(name),
			set: (name, value, opts) => {
				jar.set(name, value);
				set.push([name, value, opts]);
			},
			delete: (name) => jar.delete(name)
		}
	};
}

/** POST the callback page's body to the Google sign-in proxy. */
async function callback(body, cookieEntries) {
	const { jar, cookies } = jarOf(cookieEntries);
	const request = new Request('https://silo.cat/api/v1/user/google-auth', {
		method: 'POST',
		headers: { 'content-type': 'application/json' },
		body: JSON.stringify(body)
	});
	const event = { request, url: new URL(request.url), cookies, locals: {} };
	const route = await import('../src/routes/api/v1/user/google-auth/+server.js');
	const response = await handle({ event, resolve: (e) => route.POST(e) });
	const session = jar.get(SESSION_COOKIE.name);
	return { response, jar, sessionUser: session ? decodeToken(session)?.user : undefined };
}

/** GET /auth/google the way the sign-in button does; returns the redirect. */
async function start(query = '') {
	const { jar, set, cookies } = jarOf();
	const url = new URL('https://silo.cat/auth/google' + query);
	const route = await import('../src/routes/auth/google/+server.js');
	let location;
	try {
		const res = await route.GET({ url, cookies, request: new Request(url) });
		location = res.headers.get('location');
	} catch (thrown) {
		if (!thrown?.location) throw thrown;
		location = thrown.location;
	}
	return { location: new URL(location), jar, set };
}

const STATE = 'a'.repeat(64);
const CODE = '4/0Attacker-kept-this-code';
const REDIRECT = 'https://silo.cat/auth/callback';

test('a code with no sign-in started in this browser gets no session', async () => {
	// The attacker's link: their code, and whatever state they like.
	const { response, sessionUser } = await callback({
		code: CODE,
		state: STATE,
		redirect_uri: REDIRECT
	});
	assert.equal(response.status, 400);
	assert.equal(sessionUser, undefined);
	assert.ok(!calls.some(([, p]) => p === '/user/google-auth'), 'the code was exchanged');
});

test('a code with no state at all gets no session', async () => {
	const { response, sessionUser } = await callback(
		{ code: CODE, redirect_uri: REDIRECT },
		{ [STATE_COOKIE]: STATE }
	);
	assert.equal(response.status, 400);
	assert.equal(sessionUser, undefined);
	assert.ok(!calls.some(([, p]) => p === '/user/google-auth'));
});

test("a state that is not this browser's gets no session", async () => {
	const { response, sessionUser } = await callback(
		{ code: CODE, state: 'b'.repeat(64), redirect_uri: REDIRECT },
		{ [STATE_COOKIE]: STATE }
	);
	assert.equal(response.status, 400);
	assert.equal(sessionUser, undefined);
	assert.ok(!calls.some(([, p]) => p === '/user/google-auth'));
});

test('the sign-in button sets a state cookie and sends the same state to Google', async () => {
	const { location, jar, set } = await start('?redirect_uri=' + encodeURIComponent(REDIRECT));
	assert.equal(location.origin, 'https://accounts.google.com');
	const state = location.searchParams.get('state');
	assert.match(state ?? '', /^[0-9a-f]{64}$/);
	assert.equal(jar.get(STATE_COOKIE), state);
	const [, , opts] = set.find(([name]) => name === STATE_COOKIE);
	assert.equal(opts.httpOnly, true);
	assert.equal(opts.sameSite, 'lax');
	assert.equal(opts.secure, true);
	assert.equal(opts.path, '/');
	assert.ok(opts.maxAge > 0 && opts.maxAge <= 900);
	assert.equal(location.searchParams.get('client_id'), 'test-client.apps.googleusercontent.com');
	assert.equal(location.searchParams.get('redirect_uri'), REDIRECT);
	assert.equal(location.searchParams.get('response_type'), 'code');
	assert.equal(location.searchParams.get('scope'), 'openid email profile');

	const again = await start('?redirect_uri=' + encodeURIComponent(REDIRECT));
	assert.notEqual(again.location.searchParams.get('state'), state);
});

test('the sign-in button only sends Google back to /auth/callback', async () => {
	const { location } = await start(
		'?redirect_uri=' + encodeURIComponent('https://evil.example/steal')
	);
	assert.equal(location.searchParams.get('redirect_uri'), 'https://silo.cat/auth/callback');
	const none = await start();
	assert.equal(none.location.searchParams.get('redirect_uri'), 'https://silo.cat/auth/callback');
});

test('the browser that started the sign-in is signed in, once', async () => {
	const { location, jar } = await start('?redirect_uri=' + encodeURIComponent(REDIRECT));
	const state = location.searchParams.get('state');
	const cookie = { [STATE_COOKIE]: jar.get(STATE_COOKIE) };

	const ok = await callback({ code: CODE, state, redirect_uri: REDIRECT }, cookie);
	assert.equal(ok.response.status, 200);
	assert.equal(ok.sessionUser?.id, 'u-attacker');
	assert.equal(ok.jar.get(STATE_COOKIE), undefined, 'the state cookie outlived its sign-in');
	const sent = calls.filter(([, p]) => p === '/user/google-auth');
	assert.equal(sent.length, 1);
	assert.equal(sent[0][2].code, CODE);
	assert.equal(sent[0][2].redirect_uri, REDIRECT);
	assert.equal(sent[0][2].state, undefined);
});

test('the pages start Google sign-in through /auth/google and return the state', () => {
	const src = (p) => readFileSync(new URL('../src/routes/' + p, import.meta.url), 'utf8');
	for (const page of ['auth/signin/+page.svelte', 'auth/signup/+page.svelte']) {
		assert.ok(!src(page).includes('accounts.google.com'), `${page} builds its own Google URL`);
		assert.ok(src(page).includes('/auth/google?'), `${page} does not use /auth/google`);
	}
	const cb = src('auth/callback/+page.svelte');
	assert.match(cb, /searchParams\.get\('state'\)/);
	assert.match(cb, /\bstate\b[,\s]/);
});
