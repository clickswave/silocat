import { json } from '@sveltejs/kit';
import { ApiServerClient } from '$lib/network.js';

/**
 * POST /api/v1/user/rotate-api-key
 *
 * Issues a new API key and returns it once. Deliberately session-only: rotating
 * with the key you are rotating away is a footgun, and an attacker holding a
 * stolen key must not be able to lock the owner out by rotating it.
 */
export async function POST({ locals }) {
	const session = await locals.session.get();
	const user = session?.user;
	if (!user) return json({ error: 'Unauthorized' }, { status: 401 });

	// Reject key-authenticated callers even if the header resolved a user.
	if (!session) return json({ error: 'Session required' }, { status: 401 });

	try {
		const res = await ApiServerClient.post(
			'/user/rotate-api-key',
			{},
			{ headers: { 'X-Api-Key': user.api_key } }
		);

		const apiKey = res?.data?.data?.api_key;
		if (!apiKey) {
			return json({ error: 'Rotation failed' }, { status: 500 });
		}

		// The session carries the key for every server-side call, so it has to be
		// refreshed here or the app keeps using the key it just invalidated.
		await locals.session.user.set({ ...user, api_key: apiKey });

		return json({ success: { api_key: apiKey } });
	} catch (err) {
		const status = err?.response?.status || 500;
		return json(err?.response?.data || { error: 'Rotation failed' }, { status });
	}
}
