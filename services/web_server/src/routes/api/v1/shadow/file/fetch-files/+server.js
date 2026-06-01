import { json } from '@sveltejs/kit';
import { ApiServerClient, ApiServerRoutes } from '$lib/network.js';

export async function POST({ request, locals }) {

    // Shadow validation not strictly required for public fetch if by ID
    // But good practice if headers present.

    let body;
    try {
        body = await request.json();
    } catch {
        return json({ error: 'Invalid JSON body' }, { status: 400 });
    }

    try {
        let response = await ApiServerClient.post(ApiServerRoutes.fetchFiles, body).then(res => res.data);
        return json(response);
    } catch (err) {
        console.error('[FETCH_FILES]', err);
        return json({ error: 'Failed to fetch file' }, { status: 500 });
    }
}
