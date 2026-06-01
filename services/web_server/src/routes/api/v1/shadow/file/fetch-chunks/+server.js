import { json } from '@sveltejs/kit';
import { ApiServerClient, ApiServerRoutes } from '$lib/network.js';

export async function POST({ request, locals }) {

    let body;
    try {
        body = await request.json();
    } catch {
        return json({ error: 'Invalid JSON body' }, { status: 400 });
    }

    try {
        // Warning: network.js defines fetchChunks as /file/fetch-chunks but ApiServerRoutes might differ
        // I need to ensure network.js actually has fetchChunks mapped correctly
        let response = await ApiServerClient.post(ApiServerRoutes.fetchChunks, body).then(res => res.data);
        return json(response);
    } catch (err) {
        console.error('[FETCH_CHUNKS]', err);
        return json({ error: 'Failed to fetch chunks' }, { status: 500 });
    }
}
