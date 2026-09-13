
import { json } from '@sveltejs/kit';
import { ApiServerClient, ApiServerRoutes, proxyError } from '$lib/network.js';

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
        let payload = {
            ...body,
            user_id: sessionUser.id
        };

        // Need to add deleteFolder to ApiServerRoutes network.js if not exists
        // Checked network.js content in previous steps, it had:
        // createFolder: '/folder/create',
        // fetchFolder: '/folder/fetch',
        // But no delete endpoint mapped? Backend has /delete-folders.

        // Correct endpoint is /folder/delete (registered in folder/mod.rs)
        let response = await ApiServerClient.post('/folder/delete', payload, { headers: { 'X-Api-Key': sessionUser.api_key } });
        return json(response.data);
    } catch (err) {
        return proxyError(err, '[DELETE_FOLDER]', 'Failed to delete folder');
    }
}
