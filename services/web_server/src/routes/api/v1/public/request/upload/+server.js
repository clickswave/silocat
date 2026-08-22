import { json } from '@sveltejs/kit';
import { ApiServerClient } from '$lib/network.js';

// Public: begin an upload into a request owner's account (authorized by token).
export async function POST({ request, getClientAddress }) {
	try {
		const payload = await request.json();
		const res = await ApiServerClient.post('/file/public/request/upload', payload, {
			headers: { 'X-Client-IP': getClientAddress() }
		});
		return json({ success: { status: res.data.status, data: res.data.data } });
	} catch (err) {
		const d = err.response?.data;
		if (d) return json(d, { status: err.response.status || 400 });
		console.error('[REQ_UPLOAD_PROXY]', err);
		return json({ error: 'Internal Error' }, { status: 500 });
	}
}
