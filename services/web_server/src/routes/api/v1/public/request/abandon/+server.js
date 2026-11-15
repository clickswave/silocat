import { json } from '@sveltejs/kit';
import { ApiServerClient } from '$lib/network.js';
import { clientIpHeaders } from '$lib/server/client-ip.js';

// Public: give up an unfinished request upload so its slot on the link goes
// back (authorized by token). The upload page calls this when an upload fails.
export async function POST(event) {
	try {
		const payload = await event.request.json();
		const res = await ApiServerClient.post('/file/public/request/abandon', payload, {
			headers: clientIpHeaders(event)
		});
		return json({ success: { status: res.data.status, data: res.data.data } });
	} catch (err) {
		const d = err.response?.data;
		if (d) return json(d, { status: err.response.status || 400 });
		console.error('[REQ_ABANDON_PROXY]', err);
		return json({ error: 'Internal Error' }, { status: 500 });
	}
}
