import { json } from '@sveltejs/kit';
import { ApiServerClient } from '$lib/network.js';

export async function GET({ params, locals }) {
	let user = await locals.session.user.get();
	if (!user) {
		return json({ error: 'Unauthorized' }, { status: 401 });
	}
	try {
		const response = await ApiServerClient.get(`/user/support/${params.id}`, {
			headers: { 'X-Api-Key': user.api_key }
		});
		return json({ success: response.data });
	} catch (err) {
		if (err.response) {
			return json(err.response.data, { status: err.response.status });
		}
		console.error('[GET_SUPPORT_TICKET]', err);
		return json({ error: 'Failed to load ticket' }, { status: 500 });
	}
}
