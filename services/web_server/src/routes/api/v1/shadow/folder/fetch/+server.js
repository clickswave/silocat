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
        let response = await ApiServerClient.post(ApiServerRoutes.getFolder, body).then(res => res.data);
        return json(response);
    } catch (err) {
        console.error('[FETCH_FOLDER]', err);
        return json({ error: 'Failed to fetch folder' }, { status: 500 });
    }
}
