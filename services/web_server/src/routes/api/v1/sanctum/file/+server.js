
import { json } from '@sveltejs/kit';
import { ApiServerClient, ApiServerRoutes } from '$lib/network.js';

export async function POST({ request, locals }) {

    // 1. Check for Authenticated Session
    let sessionUser = await locals.session.user.get();

    if (!sessionUser) {
        return json({ error: 'Unauthorized' }, { status: 401 });
    }

    let body;
    try {
        body = await request.json();

        // Authenticated User: Inject user_id
        body.user_id = sessionUser.id;
        body.owner_api_key = sessionUser.api_key;
        body.storage_type = 'sanctum'; // Explicitly set storage type

    } catch (e) {
        console.log(e);
        return json({ error: 'Invalid JSON body' }, { status: 400 });
    }

    try {
        // Use the same CREATE_FILE endpoint in api_switch, but now we've injected user_id and storage_type
        let response = await ApiServerClient.post(ApiServerRoutes.createFile, body).then(res => res.data);
        return json(response);
    } catch (err) {
        console.error('[CREATE_SANCTUM_FILE]', err);
        return json({ error: 'Failed to create file metadata' }, { status: 500 });
    }
}
