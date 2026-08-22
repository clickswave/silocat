import { json } from '@sveltejs/kit';
import { ApiServerClient } from '$lib/network.js';

export async function GET({ locals }) {
	const user = await locals.session.user.get();
	if (!user) return json({ error: 'Unauthorized' }, { status: 401 });
	try {
		const res = await ApiServerClient.get('/file/request/list', {
			headers: { 'X-Api-Key': user.api_key }
		});
		return json({ success: { status: res.data.status, data: res.data.data } });
	} catch (err) {
		const d = err.response?.data;
		if (d) return json(d, { status: err.response.status || 400 });
		console.error('[REQ_LIST_PROXY]', err);
		return json({ error: 'Internal Error' }, { status: 500 });
	}
}
