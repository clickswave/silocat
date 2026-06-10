import { json } from '@sveltejs/kit';
import { ApiServerClient } from '$lib/network.js';

export async function GET({ locals }) {
	let user = await locals.session.user.get();
	if (!user) {
		return json({ error: 'Unauthorized' }, { status: 401 });
	}
	try {
		const response = await ApiServerClient.get('/user/support', {
			headers: { 'X-Api-Key': user.api_key }
		});
		return json({ success: response.data });
	} catch (err) {
		if (err.response) {
			return json(err.response.data, { status: err.response.status });
		}
		console.error('[GET_SUPPORT]', err);
		return json({ error: 'Failed to load tickets' }, { status: 500 });
	}
}

export async function POST({ request, locals }) {
	let user = await locals.session.user.get();
	if (!user) {
		return json({ error: 'Unauthorized' }, { status: 401 });
	}
	try {
		const payload = await request.json();
		const response = await ApiServerClient.post('/user/support', payload, {
			headers: { 'X-Api-Key': user.api_key }
		});
		return json({ success: response.data });
	} catch (err) {
		if (err.response) {
			return json(err.response.data, { status: err.response.status });
		}
		console.error('[POST_SUPPORT]', err);
		return json({ error: 'Failed to send message' }, { status: 500 });
	}
}
