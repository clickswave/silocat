import { json } from '@sveltejs/kit';
import { ApiServerClient } from '$lib/network.js';
import { clientIpHeaders } from '$lib/server/client-ip.js';

// POST /api/v1/user/forgot-password  { email }  -> emails a reset code.
export async function POST(event) {
	const { request } = event;
	let email;
	try {
		({ email } = await request.json());
	} catch {
		return json({ error: 'Invalid request' }, { status: 400 });
	}

	try {
		const res = await ApiServerClient.post('/user/forgot-password', { email }, { headers: clientIpHeaders(event) });
		return json({ success: res.data });
	} catch (err) {
		const status = err?.response?.status || 500;
		const data = err?.response?.data;
		console.error('[FORGOT_PW]', data || err.message);
		return json(
			{ error: data?.message || 'Could not send reset code', retry_after: data?.data?.retry_after },
			{ status }
		);
	}
}
