import { json } from '@sveltejs/kit';
import { ApiServerClient } from '$lib/network.js';
import { clientIpHeaders } from '$lib/server/client-ip.js';

// Public: what a request is for (label + message + whether it is open).
// api_switch rate-limits this per client IP before it looks the token up, so
// without the visitor's address every visitor shares one bucket.
export async function GET(event) {
	try {
		const res = await ApiServerClient.get(
			'/file/public/request/info/' + encodeURIComponent(event.params.token),
			{ headers: clientIpHeaders(event) }
		);
		return json({ success: { status: res.data.status, data: res.data.data } });
	} catch (err) {
		const d = err.response?.data;
		if (d) return json(d, { status: err.response.status || 404 });
		console.error('[REQ_INFO_PROXY]', err);
		return json({ error: 'Internal Error' }, { status: 500 });
	}
}
