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
        // body should contain { file_id, api_key }
        let response = await ApiServerClient.post(ApiServerRoutes.deleteFile, body).then(res => res.data);
        return json(response);
    } catch (err) {
        console.error('[DELETE_FILE]', err);
        const status = err.response?.status || 500;
        const data = err.response?.data || { error: 'Failed to delete file' };
        return json(data, { status });
    }
}
