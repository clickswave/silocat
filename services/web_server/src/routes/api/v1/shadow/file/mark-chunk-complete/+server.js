import { json } from '@sveltejs/kit';
import { ApiServerClient, ApiServerRoutes } from '$lib/network.js';

export async function POST({ request, locals }) {
    // Basic validation implies the user has the API key. 
    // In Shadow mode, validation is minimal as it's anonymous.
    // However, we should pass X-Api-Key header if present.
    // The previous file/+server.js validated the user using ValidateShadowUser.

    // We can reuse the validation logic if we want, or just forward.
    // Given mark-chunk-complete includes chunk_id, backend handles validity.

    let body;
    try {
        body = await request.json();
    } catch {
        return json({ error: 'Invalid JSON body' }, { status: 400 });
    }

    try {
        let response = await ApiServerClient.post(ApiServerRoutes.markChunkAsComplete, body).then(res => res.data);
        return json(response);
    } catch (err) {
        console.error('[MARK_CHUNK_COMPLETE]', err);
        return json({ error: 'Failed to mark chunk complete' }, { status: 500 });
    }
}
