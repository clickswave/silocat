// The OAuth `state` for Google sign-in.
//
// /auth/google makes a random state, keeps it in an httpOnly cookie and sends
// it to Google, which hands it back on /auth/callback. The sign-in proxy only
// trades the code for a session when the state that came back matches the
// cookie. A code someone else started (and mailed to the victim as a callback
// link) arrives without this browser's state, so it signs nobody in.
import { dev } from '$app/environment';

export const OAUTH_STATE_COOKIE = 'sc_oauth_state';
// Long enough to pick an account and consent, short enough to be useless later.
const MAX_AGE = 60 * 10;

/** 32 random bytes as hex. */
export function newOAuthState() {
	const bytes = new Uint8Array(32);
	crypto.getRandomValues(bytes);
	return Array.from(bytes, (b) => b.toString(16).padStart(2, '0')).join('');
}

/** @param {import('@sveltejs/kit').Cookies} cookies @param {string} state */
export function setOAuthState(cookies, state) {
	cookies.set(OAUTH_STATE_COOKIE, state, {
		path: '/',
		httpOnly: true,
		sameSite: 'lax',
		secure: !dev,
		maxAge: MAX_AGE
	});
}

/**
 * True when `state` is the one this browser was given. The cookie is cleared
 * either way, so a state works for one sign-in only.
 * @param {import('@sveltejs/kit').Cookies} cookies @param {unknown} state
 */
export function takeOAuthState(cookies, state) {
	const expected = cookies.get(OAUTH_STATE_COOKIE);
	cookies.delete(OAUTH_STATE_COOKIE, { path: '/' });
	if (typeof expected !== 'string' || typeof state !== 'string') return false;
	if (expected.length !== 64 || state.length !== expected.length) return false;
	let diff = 0;
	for (let i = 0; i < expected.length; i++) diff |= expected.charCodeAt(i) ^ state.charCodeAt(i);
	return diff === 0;
}
