
import { json } from '@sveltejs/kit';
import { ApiServerClient, ApiServerRoutes } from '$lib/network.js';

export async function POST({ request, locals }) {
    let user = await locals.session.user.get();
    if (!user) {
        return json({ error: 'Unauthorized' }, { status: 401 });
    }

    try {
        const payload = await request.json();
        // Inject user_id from session
        const enhancedPayload = {
            ...payload,
            user_id: user.id
        };

        let response = await ApiServerClient.post('/billing/order', enhancedPayload, { headers: { 'X-Api-Key': user.api_key } })
            .then(res => res.data);
        console.log("response", response);
        return json({ success: response.data });
    } catch (err) {
        console.error('[POST_BILLING_ORDER]', err?.response?.data || err.message);
        const status = err?.response?.status || 500;
        const message = err?.response?.data?.message || 'Failed to create order';
        const data = err?.response?.data || {};
        return json({ error: 'Failed to create order', message, ...data }, { status });
    }
}
