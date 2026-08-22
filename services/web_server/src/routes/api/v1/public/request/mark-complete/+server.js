import { json } from '@sveltejs/kit';
import { ApiServerClient } from '$lib/network.js';

// Public: mark a request-upload chunk complete (authorized by token).
export async function POST({ request, getClientAddress }) {
	try {
		const payload = await request.json();
		const res = await ApiServerClient.post('/file/public/request/mark-complete', payload, {
			headers: { 'X-Client-IP': getClientAddress() }
		});
		return json({ success: { status: res.data.status, data: res.data.data } });
	} catch (err) {
		const d = err.response?.data;
		if (d) return json(d, { status: err.response.status || 400 });
		console.error('[REQ_MARK_COMPLETE_PROXY]', err);
		return json({ error: 'Internal Error' }, { status: 500 });
	}
}
