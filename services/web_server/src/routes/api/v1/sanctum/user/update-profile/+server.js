import { json } from '@sveltejs/kit';
import { ApiServerClient, ApiServerRoutes } from '$lib/network.js';

export async function POST({ request, locals }) {
    let user = await locals.session.user.get();
    if (!user) {
        return json({ error: 'Unauthorized' }, { status: 401 });
    }

    try {
        const payload = await request.json();
        const response = await ApiServerClient.post('/user/update-profile', payload, {
            headers: { 'X-Api-Key': user.api_key }
        });

        return json({ success: response.data });
    } catch (err) {
        console.error('[POST_SANCTUM_UPDATE_PROFILE]', err);
        return json({ error: 'Failed to update profile' }, { status: 500 });
    }
}
