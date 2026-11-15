// A caller that proved itself with X-Api-Key must never come away with a
// session cookie, and must not be able to change the account's email or
// password. Run with `npm test`.
import { test, beforeEach } from 'node:test';
import assert from 'node:assert/strict';
import * as nodeModule from 'node:module';

const here = new URL('./stubs/', import.meta.url).href;
const lib = new URL('../src/lib/', import.meta.url).href;
const env =
	'data:text/javascript,' +
	encodeURIComponent('export const env = { JWT_SECRET: "test-jwt-secret" };');
const appEnv = 'data:text/javascript,' + encodeURIComponent('export const dev = true;');
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
const { createSession, SESSION_COOKIE } = await import('../src/lib/auth.js');
const { decodeToken } = await import('../src/lib/jwt.js');

const routes = {
	'/api/v1/user/request-email-change': '../src/routes/api/v1/user/request-email-change/+server.js',
	'/api/v1/user/confirm-email-change': '../src/routes/api/v1/user/confirm-email-change/+server.js',
	'/api/v1/user/change-password': '../src/routes/api/v1/user/change-password/+server.js',
	'/api/v1/user/verify-email': '../src/routes/api/v1/user/verify-email/+server.js',
	'/api/v1/billing/verify': '../src/routes/api/v1/billing/verify/+server.js'
};

const victim = { id: 'u-victim', email: 'owner@example.com', api_key: 'victim-key', password_set: true };
const other = { id: 'u-other', email: 'other@example.com', api_key: 'other-key', password_set: true };

/** Every call that reached api_switch, as [method, path, X-Api-Key]. */
let calls = [];

beforeEach(() => {
	calls = [];
	globalThis.fakeApi = {
		get: async (path, opts) => {
			const key = opts?.headers?.['X-Api-Key'];
			calls.push(['get', path, key]);
			const user = [victim, other].find((u) => u.api_key === key);
			if (path === '/user/info' && user) return { status: 200, data: { data: user } };
			throw { response: { status: 401, data: { message: 'Unauthorized' } } };
		},
		post: async (path, _body, opts) => {
			const key = opts?.headers?.['X-Api-Key'];
			calls.push(['post', path, key]);
			const user = [victim, other].find((u) => u.api_key === key) ?? victim;
			if (path === '/user/confirm-email-change') {
				return { status: 200, data: { data: { ...user, email: 'attacker@evil.example' } } };
			}
			return { status: 200, data: { status: 200, data: { ...user } } };
		}
	};
});

/** Run one request through the hook and the route, the way SvelteKit would. */
async function send(path, { key, session, body } = {}) {
	const jar = new Map();
	if (session) jar.set(SESSION_COOKIE.name, createSession({ user: session }).value);
	const set = [];
	const cookies = {
		get: (name) => jar.get(name),
		set: (name, value) => {
			jar.set(name, value);
			set.push(name);
		},
		delete: (name) => jar.delete(name)
	};
	const headers = { 'content-type': 'application/json' };
	if (key) headers['x-api-key'] = key;
	const request = new Request('https://silo.cat' + path, {
		method: 'POST',
		headers,
		body: JSON.stringify(body ?? {})
	});
	const event = { request, url: new URL(request.url), cookies, locals: {} };
	const route = await import(routes[path]);
	const response = await handle({ event, resolve: (e) => route.POST(e) });
	const cookie = jar.get(SESSION_COOKIE.name);
	return { response, set, sessionUser: cookie ? decodeToken(cookie)?.user : undefined };
}

test('a key alone cannot start an email change', async () => {
	const { response, set } = await send('/api/v1/user/request-email-change', {
		key: 'victim-key',
		body: { email: 'attacker@evil.example' }
	});
	assert.equal(response.status, 401);
	assert.deepEqual(set, []);
	assert.ok(!calls.some(([m, p]) => m === 'post' && p === '/user/request-email-change'));
});

test('a key alone cannot confirm an email change or get a session for it', async () => {
	const { response, set } = await send('/api/v1/user/confirm-email-change', {
		key: 'victim-key',
		body: { otp: '123456' }
	});
	assert.equal(response.status, 401);
	assert.deepEqual(set, []);
	assert.ok(!calls.some(([m, p]) => m === 'post' && p === '/user/confirm-email-change'));
});

test('a key alone cannot set or change the password', async () => {
	const { response, set } = await send('/api/v1/user/change-password', {
		key: 'victim-key',
		body: { new_password: 'Attacker-Chose-This-1' }
	});
	assert.equal(response.status, 401);
	assert.deepEqual(set, []);
	assert.ok(!calls.some(([m, p]) => m === 'post' && p === '/user/change-password'));
});

test('a key caller verifying a payment or an email gets no session cookie', async () => {
	for (const path of ['/api/v1/billing/verify', '/api/v1/user/verify-email']) {
		const { response, set } = await send(path, { key: 'victim-key', body: { otp: '1' } });
		assert.equal(response.status, 200, path);
		assert.deepEqual(set, [], `${path} set a cookie for a key caller`);
	}
});

test('a key does not overwrite the user in a session the caller already has', async () => {
	// The attacker's own signed-in browser, plus the victim's key.
	const { sessionUser } = await send('/api/v1/billing/verify', {
		key: 'victim-key',
		session: other,
		body: {}
	});
	assert.equal(sessionUser?.id, 'u-other');
});

test('an email change with a key and a session acts on the session user', async () => {
	await send('/api/v1/user/request-email-change', {
		key: 'victim-key',
		session: other,
		body: { email: 'new@example.net', current_password: 'x' }
	});
	const sent = calls.filter(([m, p]) => m === 'post' && p === '/user/request-email-change');
	assert.deepEqual(sent, [['post', '/user/request-email-change', 'other-key']]);
});

test('a signed-in user still changes their email and the session follows', async () => {
	const start = await send('/api/v1/user/request-email-change', {
		session: victim,
		body: { email: 'new@example.net', current_password: 'pw' }
	});
	assert.equal(start.response.status, 200);
	const done = await send('/api/v1/user/confirm-email-change', {
		session: victim,
		body: { otp: '123456' }
	});
	assert.equal(done.response.status, 200);
	assert.equal(done.sessionUser?.email, 'attacker@evil.example');
});

test('a signed-in user still changes their password', async () => {
	const { response } = await send('/api/v1/user/change-password', {
		session: victim,
		body: { current_password: 'a', new_password: 'B-new-password-1' }
	});
	assert.equal(response.status, 200);
});
