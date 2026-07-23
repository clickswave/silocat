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
        // Inject the caller's identity so the backend can enforce ownership:
        // the logged-in session key, else the anonymous (shadow) key. Absent =>
        // only public files are downloadable.
        const sessionUser = await locals.session.user.get();
        const api_key = sessionUser?.api_key || request.headers.get('X-Api-Key') || undefined;
        let response = await ApiServerClient.post(ApiServerRoutes.fetchChunks, { ...body, api_key }, { headers: { 'X-Api-Key': api_key } }).then(res => res.data);
        return json(response);
    } catch (err) {
        console.error('[FETCH_CHUNKS]', err);
        return json({ error: 'Failed to fetch chunks' }, { status: 500 });
    }
}
