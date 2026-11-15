import { json } from '@sveltejs/kit';
import { ApiServerClient } from '$lib/network.js';

// POST /api/v1/user/change-password  { current_password?, new_password }
// current_password is omitted when a Google account sets a password for the
// first time. Updates the session's password_set flag on success.
export async function POST({ request, locals }) {
	// Session only, like rotate-api-key. An account that never set a password
	// takes a new one with no current password, and whoever then knows it can
	// sign in. An API key is handed to scripts and CI, so it must not be enough
	// for that, and with both a key and a cookie the cookie's account is the
	// one acted on.
	const session = await locals.session.get();
	const user = session?.user;
	if (!user) {
		return json({ error: 'Session required' }, { status: 401 });
	}

	let payload;
	try {
		payload = await request.json();
	} catch {
		return json({ error: 'Invalid request' }, { status: 400 });
	}

	try {
		const res = await ApiServerClient.post('/user/change-password', payload, {
			headers: { 'X-Api-Key': user.api_key }
		});
		await locals.session.user.update({ key: 'password_set', value: true });
		return json({ success: res.data });
	} catch (err) {
		const status = err?.response?.status || 500;
		const data = err?.response?.data;
		console.error('[CHANGE_PW]', data || err.message);
		return json({ error: data?.message || 'Could not update password', errors: data?.errors }, { status });
	}
}
