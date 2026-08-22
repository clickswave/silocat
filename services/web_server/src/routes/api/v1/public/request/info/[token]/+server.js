import { json } from '@sveltejs/kit';
import { ApiServerClient } from '$lib/network.js';

// Public: what a request is for (label + message + whether it is open).
export async function GET({ params }) {
	try {
		const res = await ApiServerClient.get(
			'/file/public/request/info/' + encodeURIComponent(params.token)
		);
		return json({ success: { status: res.data.status, data: res.data.data } });
	} catch (err) {
		const d = err.response?.data;
		if (d) return json(d, { status: err.response.status || 404 });
		console.error('[REQ_INFO_PROXY]', err);
		return json({ error: 'Internal Error' }, { status: 500 });
	}
}
