import { redirect } from '@sveltejs/kit';
import { ApiServerClient } from '$lib/network.js';

export const load = async ({ locals, url }) => {
	try {
		let session = await locals.session.get();
		let user = session?.user;
		let storage = session?.storage;

		// Auth guard: the /home area requires a session. Without one, send the
		// visitor to sign in instead of rendering a broken, data-less page.
		if (!user && url.pathname.startsWith('/home')) {
			return redirect(302, '/auth/signin');
		}

		// If a logged-in user has been banned mid-session, log them out on their
		// next navigation into /home. One lightweight check; ignore transient errors.
		if (user && url.pathname.startsWith('/home')) {
			try {
				await ApiServerClient.get('/user/info', { headers: { 'X-Api-Key': user.api_key } });
			} catch (e) {
				const banned =
					e?.response?.status === 403 &&
					(e.response.data?.data?.banned || /banned/i.test(e.response.data?.message || ''));
				if (banned) {
					await locals.session.delete();
					return redirect(302, '/auth/signin?banned=1');
				}
			}
		}

		let totalAvailableSpace = Number(user?.default_storage_bytes) || 0;
		if (user?.subscription?.additional_space) {
			totalAvailableSpace += Number(user.subscription.additional_space);
		}

		if (user) {
			user.totalAvailableSpace = totalAvailableSpace;

			// Enforce email verification
			if (!user.email_verified) {
				if (!url.pathname.startsWith('/home/pending-actions') && !url.pathname.startsWith('/auth')) {
					return redirect(302, '/home/pending-actions');
				}
			} else {
				if (url.pathname.startsWith('/home/pending-actions')) {
					return redirect(302, '/home');
				}
			}
		}

		// Never serialize the account api_key to the client. Whatever a load()
		// returns lands in the page hydration payload, which is readable by any
		// client-side JS, browser extension, or error logger; the api_key is a
		// long-lived, non-expiring capability credential and must stay server-side.
		// Every /api/v1 proxy attaches it from the session cookie, so the browser
		// never needs it. (The raw key lives only in the httpOnly session cookie.)
		let clientUser = user;
		if (user) {
			const { api_key, ...rest } = user;
			clientUser = rest;
		}

		return {
			user: clientUser,
			storage
		};
	} catch (e) {
		if (e.status && e.location) {
			throw e;
		}
		console.error("Layout load error:", e);
		return { user: null, storage: null };
	}
};
