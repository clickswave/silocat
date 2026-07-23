
import { json } from '@sveltejs/kit';
import { ApiServerClient } from '$lib/network.js';

export async function POST({ request, locals }) {
    // Promo check is public-ish but restricted to logged in users in UI
    // specific user injection not needed for checking validity

    let user = await locals.session.user.get();
    if (!user) {
        return json({ error: 'Unauthorized' }, { status: 401 });
    }

    try {
        const payload = await request.json();

        let response = await ApiServerClient.post('/billing/check-promo', payload, { headers: { 'X-Api-Key': user.api_key } })
            .then(res => res.data);

        // Return the full backend response as is
        return json(response);
    } catch (err) {
        console.error('[POST_BILLING_CHECK_PROMO]', err?.response?.data || err.message);
        // If backend returns 400, pass it through
        if (err.response) {
            return json(err.response.data, { status: err.response.status });
        }
        return json({ error: 'Failed to check promo code' }, { status: 500 });
    }
}
