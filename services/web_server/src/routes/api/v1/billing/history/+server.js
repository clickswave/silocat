import { json } from '@sveltejs/kit';
import { ApiServerClient } from '$lib/network.js';

export async function GET({ locals }) {
	const user = await locals.session.user.get();
	if (!user) {
		return json({ error: 'Unauthorized' }, { status: 401 });
	}

	try {
		const response = await ApiServerClient.get('/billing/history', {
			headers: { 'X-Api-Key': user.api_key }
		}).then((res) => res.data);
		return json(response);
	} catch (err) {
		const status = err?.response?.status || 500;
		return json(err?.response?.data || { error: 'Failed to fetch billing history' }, { status });
	}
}
