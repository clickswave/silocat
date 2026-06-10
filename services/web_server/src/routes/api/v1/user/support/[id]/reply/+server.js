import { json } from '@sveltejs/kit';
import { ApiServerClient } from '$lib/network.js';

export async function POST({ params, request, locals }) {
	let user = await locals.session.user.get();
	if (!user) {
		return json({ error: 'Unauthorized' }, { status: 401 });
	}
	try {
		const payload = await request.json();
		const response = await ApiServerClient.post(`/user/support/${params.id}/reply`, payload, {
			headers: { 'X-Api-Key': user.api_key }
		});
		return json({ success: response.data });
	} catch (err) {
		if (err.response) {
			return json(err.response.data, { status: err.response.status });
		}
		console.error('[POST_SUPPORT_REPLY]', err);
		return json({ error: 'Failed to add reply' }, { status: 500 });
	}
}
