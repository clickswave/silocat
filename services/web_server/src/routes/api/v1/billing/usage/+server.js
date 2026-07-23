
import { json } from '@sveltejs/kit';
import { ApiServerClient } from '$lib/network.js';

export async function GET({ locals }) {
    let user = await locals.session.user.get();
    if (!user) {
        return json({ error: 'Unauthorized' }, { status: 401 });
    }

    try {
        let response = await ApiServerClient.get(`/billing/usage?user_id=${user.id}`, { headers: { 'X-Api-Key': user.api_key } })
            .then(res => res.data);

        return json(response);
    } catch (err) {
        console.error('[GET_BILLING_USAGE]', err?.response?.data || err.message);
        if (err.response) {
            return json(err.response.data, { status: err.response.status });
        }
        return json({ error: 'Failed to fetch usage' }, { status: 500 });
    }
}
