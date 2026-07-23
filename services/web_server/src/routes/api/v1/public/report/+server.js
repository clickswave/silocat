import { json } from '@sveltejs/kit';
import { ApiServerClient, ApiServerRoutes } from '$lib/network.js';

// POST /api/v1/public/report  { share_token?, reason, details? }
// Public abuse-report proxy for the share pages. No session required; the
// backend rate-limits per client IP, so forward the real IP.
export async function POST({ request }) {
	let body;
	try {
		body = await request.json();
	} catch {
		return json({ error: 'Invalid JSON body' }, { status: 400 });
	}

	try {
		const headers = {};
		const cfip = request.headers.get('cf-connecting-ip');
		if (cfip) headers['CF-Connecting-IP'] = cfip;
		const res = await ApiServerClient.post(ApiServerRoutes.report, body, { headers });
		return json(res.data);
	} catch (err) {
		const status = err?.response?.status || 500;
		return json(err?.response?.data || { error: 'Could not submit report' }, { status });
	}
}
