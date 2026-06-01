
import { json } from '@sveltejs/kit';
import { ApiServerClient, ApiServerRoutes } from '$lib/network.js';

export async function POST({ request }) {
    let body;
    try {
        body = await request.json();
    } catch {
        return json({ error: 'Invalid JSON body' }, { status: 400 });
    }

    try {
        let response = await ApiServerClient.post(ApiServerRoutes.fetchResource, body).then(res => res.data);
        return json(response);
    } catch (err) {
        console.error('[FETCH_RESOURCE]', err);
        return json({ error: 'Failed to fetch resource' }, { status: 500 });
    }
}
