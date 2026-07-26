
import { json } from '@sveltejs/kit';
import { ApiServerClient } from '$lib/network.js';
import { clientIpHeaders } from '$lib/server/client-ip.js';

export async function POST(event) {
    const { request, locals } = event;
    let user = await locals.session.user.get();
    if (!user) {
        return json({ error: 'Unauthorized' }, { status: 401 });
    }

    try {
        const payload = {}; // empty payload
        const response = await ApiServerClient.post('/user/resend-verification', payload, {
            headers: {
                ...clientIpHeaders(event),
                'X-Api-Key': user.api_key
            }
        });

        return json({ success: true });

    } catch (err) {
        if (err.response) {
            return json(err.response.data, { status: err.response.status });
        }
        console.error('[POST_RESEND_VERIFICATION]', err);
        return json({ error: 'Failed to resend verification code' }, { status: 500 });
    }
}
