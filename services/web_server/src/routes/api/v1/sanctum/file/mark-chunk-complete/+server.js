
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
        // Mark chunk complete
        // We might need to inject api_key if the backend requires it for ownership check
        // But for marking chunk, it usually checks chunk_id. 
        // Adding owner_api_key just in case backend validates ownership heavily.
        let payload = {
            ...body,
            owner_api_key: sessionUser.api_key
        };

        let response = await ApiServerClient.post('/file/mark-chunk-complete', payload, { headers: { 'X-Api-Key': sessionUser.api_key } }).then(res => res.data);
        return json(response);
    } catch (err) {
        console.error('[MARK_CHUNK_COMPLETE]', err);
        return json({ error: 'Failed to mark chunk complete' }, { status: 500 });
    }
}
