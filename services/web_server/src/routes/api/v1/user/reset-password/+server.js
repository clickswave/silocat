import { json } from '@sveltejs/kit';
import { ApiServerClient } from '$lib/network.js';
import { clientIpHeaders } from '$lib/server/client-ip.js';

// POST /api/v1/user/reset-password  { email, otp, new_password }
// On success the user is logged straight in (session established).
export async function POST(event) {
	const { request, locals } = event;
	let payload;
	try {
		payload = await request.json();
	} catch {
		return json({ error: 'Invalid request' }, { status: 400 });
	}

	try {
		const res = await ApiServerClient.post('/user/reset-password', payload, { headers: clientIpHeaders(event) });
		const user = res.data?.data?.user;
		if (user) {
			await locals.session.user.set(user);
		}
		return json({ success: res.data });
	} catch (err) {
		const status = err?.response?.status || 500;
		const data = err?.response?.data;
		console.error('[RESET_PW]', data || err.message);
		return json({ error: data?.message || 'Could not reset password' }, { status });
	}
}
