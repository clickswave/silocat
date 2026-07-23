
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
        let payload = {
            ...body,
            user_id: sessionUser.id
        };

        // Need to add deleteFolder to ApiServerRoutes network.js if not exists
        // Wait, network.js had createFolder twice but no deleteFolder in snippet?
        // Checked network.js content in previous steps, it had:
        // createFolder: '/folder/create',
        // fetchFolder: '/folder/fetch',
        // But no delete endpoint mapped? Backend has /delete-folders.
        // I will use direct string or add to network.js later.
        // Let's assume I check/add it.

        // Correct endpoint is /folder/delete (registered in folder/mod.rs)
        let response = await ApiServerClient.post('/folder/delete', payload, { headers: { 'X-Api-Key': sessionUser.api_key } });
        return json(response.data);
    } catch (err) {
        console.error('[DELETE_FOLDER]', err);
        return json({ error: 'Failed to delete folder' }, { status: 500 });
    }
}
