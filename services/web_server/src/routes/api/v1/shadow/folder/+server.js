import { json } from '@sveltejs/kit';
import { ApiServerClient, ApiServerRoutes } from '$lib/network.js';

export async function POST({ request, locals }) {
    let sessionUser = await locals.session.user.get();

    // For SHADOW route, we allow anonymous if API Key is present
    let apiKey = request.headers.get('x-api-key');

    let body;
    try {
        body = await request.json();
        if (body.api_key) apiKey = body.api_key; // Allow body override
    } catch {
        return json({ error: 'Invalid JSON body' }, { status: 400 });
    }

    if (!sessionUser && !apiKey) {
        return json({ error: 'Unauthorized: Session or API Key required' }, { status: 401 });
    }

    try {
        let payload = {
            ...body
        };

        if (sessionUser) {
            payload.user_id = sessionUser.id;
        }

        if (apiKey) {
            payload.owner_api_key = apiKey;
        } else if (!sessionUser) {
            console.log('[SHADOW_CREATE_FOLDER] No API Key or Session found');
        }

        let response = await ApiServerClient.post(ApiServerRoutes.createFolder, payload, { headers: { 'X-Api-Key': sessionUser?.api_key || apiKey || undefined } }).then(res => res.data);
        return json(response);
    } catch (err) {
        console.error('[CREATE_FOLDER]', err);
        return json({ error: 'Failed to create folder' }, { status: 500 });
    }
}
