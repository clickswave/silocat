import { json } from '@sveltejs/kit';
import { ApiServerClient, ApiServerRoutes, proxyError } from '$lib/network.js';

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
        return proxyError(err, '[POST_SANCTUM_UPDATE_PROFILE]', 'Failed to update profile');
    }
}
