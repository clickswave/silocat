import { json } from '@sveltejs/kit';
import { ApiServerClient, ApiServerRoutes } from '$lib/network.js';
import { clientIpHeaders } from '$lib/server/client-ip.js';

// POST /api/v1/public/report  { share_token?, reason, details? }
// Public abuse-report proxy for the share pages. No session required; the
// backend rate-limits per client IP, so forward the real IP.
export async function POST(event) {
	const { request } = event;
	let body;
	try {
		body = await request.json();
	} catch {
		return json({ error: 'Invalid JSON body' }, { status: 400 });
	}

	try {
		const res = await ApiServerClient.post(ApiServerRoutes.report, body, {
			headers: clientIpHeaders(event)
		});
		return json(res.data);
	} catch (err) {
		const status = err?.response?.status || 500;
		return json(err?.response?.data || { error: 'Could not submit report' }, { status });
	}
}
