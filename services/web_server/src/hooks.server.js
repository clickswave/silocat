import { decodeToken } from '$lib/jwt.js';
import { createSession, SESSION_COOKIE } from '$lib/auth.js';

export const handle = async ({ event, resolve }) => {
	// SESSION MANAGEMENT FUNCTIONS
	let session = {
		// get session data
		get: async () => {
			let sessionCookie = event.cookies.get(SESSION_COOKIE.name);
			return decodeToken(sessionCookie);
		}, // set session cookie data
		set: async (sessionData) => {
			let cookie = createSession(sessionData);
			event.cookies.set(cookie.name, cookie.value, cookie.options);
			return true;
		}, // delete session cookie
		delete: async () => {
			event.cookies.delete(SESSION_COOKIE.name, { path: SESSION_COOKIE.path });
			return true;
		}
	};

	// Programmatic callers authenticate with `X-Api-Key` instead of the session
	// cookie. Resolving it here rather than in each route means every existing
	// /api/v1 handler accepts both mechanisms with no change: they all ask for
	// `locals.session.user.get()`, which now means "the authenticated caller",
	// however they proved who they are.
	//
	// Scoped to /api/v1 on purpose. Page loads and form actions stay
	// cookie-only, so a stolen key can never be replayed against an HTML route
	// to render someone's dashboard.
	let apiKeyUser;
	let apiKeyResolved = false;

	const resolveApiKeyUser = async () => {
		// Cache per request: a handler may ask for the caller several times and
		// each miss would be another round trip to api_switch.
		if (apiKeyResolved) return apiKeyUser;
		apiKeyResolved = true;

		const key = event.request.headers.get('x-api-key');
		if (!key || !event.url.pathname.startsWith('/api/v1/')) return undefined;

		try {
			const { ApiServerClient } = await import('$lib/network.js');
			const res = await ApiServerClient.get('/user/info', {
				headers: { 'X-Api-Key': key }
			});
			apiKeyUser = res?.data?.data?.user ?? res?.data?.data ?? undefined;
		} catch {
			// A bad key is simply not authenticated; routes return their own 401.
			apiKeyUser = undefined;
		}
		return apiKeyUser;
	};

	let sessionUser = {
		// The authenticated caller: API key first, then the session cookie.
		get: async () => {
			const viaKey = await resolveApiKeyUser();
			if (viaKey) return viaKey;
			let sess = await session.get();
			return sess?.user;
		}, // set user data in session
		set: async (userData) => {
			let sessionData = await session.get();
			let cookie;
			if (!sessionData) {
				cookie = createSession({ user: userData });
			} else {
				cookie = createSession({
					...sessionData, user: userData
				});
			}
			event.cookies.set(cookie.name, cookie.value, cookie.options);
			return true;
		}, // update one key in user data
		update: async ({ key, value }) => {
			let sessionData = await session.get();
			let cookie;
			if (!sessionData) {
				return false;
			} else {
				if (!sessionData['user']) {
					return false;
				} else {
					cookie = createSession({
						...sessionData, user: {
							...sessionData.user, [key]: value
						}
					});
				}
			}
			event.cookies.set(cookie.name, cookie.value, cookie.options);
			return true;
		}
	};

	let sessionSubscription = {
		// get user data from session
		get: async () => {
			let sess = await session.get();
			return sess?.subscriptions;
		}, // set user data in session
		set: async (subscriptionData) => {
			let sessionData = await session.get();

			let cookie = createSession({
				...sessionData, subscription: subscriptionData
			});

			event.cookies.set(cookie.name, cookie.value, cookie.options);
			return true;
		}
	};

	event.locals.session = {
		...session, user: sessionUser, subscription: sessionSubscription
	};

	const response = await resolve(event);
	// Staging is gated by Cloudflare Access; this is a belt-and-suspenders
	// backstop so the pre-prod site is never indexed even if a link leaks.
	if (event.url.hostname.startsWith('staging.')) {
		response.headers.set('X-Robots-Tag', 'noindex, nofollow');
	}
	return response;
};
