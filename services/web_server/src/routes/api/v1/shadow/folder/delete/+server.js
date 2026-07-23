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
        // body should contain { folder_id, api_key }
        const sessionUser = await locals.session.user.get();
        const apiKey = sessionUser?.api_key || request.headers.get('X-Api-Key') || undefined;
        let response = await ApiServerClient.post(ApiServerRoutes.deleteFolder, body, { headers: { 'X-Api-Key': apiKey } }).then(res => res.data);
        return json(response);
    } catch (err) {
        console.error('[DELETE_FOLDER]', err);
        const status = err.response?.status || 500;
        const data = err.response?.data || { error: 'Failed to delete folder' };
        return json(data, { status });
    }
}
