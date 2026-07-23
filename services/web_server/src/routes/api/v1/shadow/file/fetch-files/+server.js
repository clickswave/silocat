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
        // Inject caller identity for backend ownership checks (session or shadow key).
        const sessionUser = await locals.session.user.get();
        const api_key = sessionUser?.api_key || request.headers.get('X-Api-Key') || undefined;
        let response = await ApiServerClient.post(ApiServerRoutes.fetchFiles, { ...body, api_key }, { headers: { 'X-Api-Key': api_key } }).then(res => res.data);
        return json(response);
    } catch (err) {
        console.error('[FETCH_FILES]', err);
        return json({ error: 'Failed to fetch file' }, { status: 500 });
    }
}
