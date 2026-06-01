
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
        // Delete file
        let payload = {
            ...body,
            api_key: sessionUser.api_key // Backend usually expects 'api_key' param for delete auth
        };

        // Using existing delete-files endpoint
        let response = await ApiServerClient.post('/file/delete-files', payload).then(res => res.data);
        return json(response);
    } catch (err) {
        console.error('[DELETE_FILE]', err);
        return json({ error: 'Failed to delete file' }, { status: 500 });
    }
}
