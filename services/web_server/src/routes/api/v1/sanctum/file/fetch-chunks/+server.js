
import { json } from '@sveltejs/kit';
import { ApiServerClient, ApiServerRoutes } from '$lib/network.js';

export async function POST({ request, locals }) {
    let sessionUser = await locals.session.user.get();
    if (!sessionUser) return json({ error: 'Unauthorized' }, { status: 401 });

    let body;
    try {
        body = await request.json();
    } catch (e) {
        return json({ error: 'Invalid JSON body' }, { status: 400 });
    }

    try {
        // Fetch chunks
        // We inject owner_api_key for access control if backend requires it
        let payload = {
            ...body,
            owner_api_key: sessionUser.api_key
        };

        let response = await ApiServerClient.post('/file/fetch-chunks', payload).then(res => res.data);
        return json(response);
    } catch (err) {
        console.error('[FETCH_CHUNKS]', err);
        return json({ error: 'Failed to fetch chunks' }, { status: 500 });
    }
}
