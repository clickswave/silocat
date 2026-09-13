
import { json } from '@sveltejs/kit';
import { ApiServerClient, ApiServerRoutes, proxyError } from '$lib/network.js';

export async function GET({ locals }) {
    let sessionUser = await locals.session.user.get();
    if (!sessionUser) {
        return json({ error: 'Unauthorized' }, { status: 401 });
    }

    try {
        // Identity is proven by the session's api key (validate_token on the
        // backend), not a body user_id, so a caller can only read their own stats.
        let response = await ApiServerClient.post(ApiServerRoutes.fetchStorageStats, {}, {
            headers: { 'X-Api-Key': sessionUser.api_key }
        }).then(res => res.data);

        return json({ success: response.data });
    } catch (err) {
        return proxyError(err, '[GET_STORAGE_STATS]', 'Failed to fetch storage stats');
    }
}
