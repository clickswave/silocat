import { json } from '@sveltejs/kit';
import { ApiServerClient } from '$lib/network.js';

export async function POST({ request, locals }) {
	// Session only, like rotate-api-key. A new address that a code confirms
	// is verified, and a password reset sent there then owns the account. An
	// API key is handed to scripts and CI, so it must not be enough to take the
	// account over, and with both a key and a cookie the cookie's account is
	// the one acted on.
	const session = await locals.session.get();
	const user = session?.user;
	if (!user) {
		return json({ error: 'Session required' }, { status: 401 });
	}
	try {
		const payload = await request.json();
		const response = await ApiServerClient.post('/user/request-email-change', payload, {
			headers: { 'X-Api-Key': user.api_key }
		});
		return json({ success: response.data });
	} catch (err) {
		if (err.response) {
			return json(err.response.data, { status: err.response.status });
		}
		console.error('[POST_REQUEST_EMAIL_CHANGE]', err);
		return json({ error: 'Failed to request email change' }, { status: 500 });
	}
}
