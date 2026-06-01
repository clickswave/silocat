
import { json } from '@sveltejs/kit';
import { ApiServerClient, ApiServerRoutes } from '$lib/network.js';

export async function GET({ locals }) {
    let sessionUser = await locals.session.user.get();
    if (!sessionUser) {
        return json({ error: 'Unauthorized' }, { status: 401 });
    }

    try {
        let response = await ApiServerClient.post(ApiServerRoutes.fetchStorageStats, {
            user_id: sessionUser.id
        }).then(res => res.data);

        return json({ success: response.data });
    } catch (err) {
        console.error('[GET_STORAGE_STATS]', err);
        return json({ error: 'Failed to fetch storage stats' }, { status: 500 });
    }
}
