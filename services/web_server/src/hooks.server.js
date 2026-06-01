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

	let sessionUser = {
		// get user data from session
		get: async () => {
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

	return await resolve(event);
};
