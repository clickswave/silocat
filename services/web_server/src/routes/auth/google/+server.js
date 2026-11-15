import { redirect } from '@sveltejs/kit';
import { env } from '$env/dynamic/private';
import { newOAuthState, setOAuthState } from '$lib/server/oauthState.js';

/**
 * Where Google sends the user back. The page passes its own origin's callback,
 * since behind a proxy this server may not know the public origin; anything
 * that is not an http(s) /auth/callback falls back to this request's origin.
 * Google still checks the result against the client's registered URIs.
 * @param {URL} url
 */
function callbackUri(url) {
	const asked = url.searchParams.get('redirect_uri');
	if (asked) {
		try {
			const u = new URL(asked);
			if (
				(u.protocol === 'https:' || u.protocol === 'http:') &&
				u.pathname === '/auth/callback' &&
				!u.search &&
				!u.hash &&
				!u.username &&
				!u.password
			) {
				return u.origin + '/auth/callback';
			}
		} catch {
			// Not a URL; use this request's origin.
		}
	}
	return url.origin + '/auth/callback';
}

/** Starts Google sign-in: sets the state cookie, then hands off to Google. */
export function GET({ url, cookies }) {
	const clientId = env.OAUTH_ID_GOOGLE;
	if (!clientId) redirect(303, '/auth/signin');

	const state = newOAuthState();
	setOAuthState(cookies, state);

	const google = new URL('https://accounts.google.com/o/oauth2/v2/auth');
	google.searchParams.set('client_id', clientId);
	google.searchParams.set('redirect_uri', callbackUri(url));
	google.searchParams.set('response_type', 'code');
	google.searchParams.set('scope', 'openid email profile');
	google.searchParams.set('state', state);
	redirect(303, google.href);
}
