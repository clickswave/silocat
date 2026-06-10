import { json } from '@sveltejs/kit';
import { ApiServerClient } from '$lib/network.js';

export async function POST({ request, locals }) {
	let user = await locals.session.user.get();
	if (!user) {
		return json({ error: 'Unauthorized' }, { status: 401 });
	}
	try {
		const payload = await request.json();
		const response = await ApiServerClient.post('/user/confirm-email-change', payload, {
			headers: { 'X-Api-Key': user.api_key }
		});
		// Backend returns fresh UserTokenData (with the new, verified email).
		if (response.status === 200 && response.data?.data) {
			await locals.session.user.set(response.data.data);
		}
		return json({ success: response.data });
	} catch (err) {
		if (err.response) {
			return json(err.response.data, { status: err.response.status });
		}
		console.error('[POST_CONFIRM_EMAIL_CHANGE]', err);
		return json({ error: 'Failed to confirm email change' }, { status: 500 });
	}
}
