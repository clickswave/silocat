
import { json } from '@sveltejs/kit';
import { ApiServerClient } from '$lib/network.js';

export async function POST({ request, locals }) {
    let user = await locals.session.user.get();
    if (!user) {
        return json({ error: 'Unauthorized' }, { status: 401 });
    }

    try {
        const payload = {}; // empty payload
        const response = await ApiServerClient.post('/user/resend-verification', payload, {
            headers: {
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
