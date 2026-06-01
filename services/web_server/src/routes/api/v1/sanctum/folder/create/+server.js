
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

        let response = await ApiServerClient.post(ApiServerRoutes.createFolder, payload).then(res => res.data);
        return json(response);
    } catch (err) {
        console.error('[CREATE_FOLDER]', err);
        return json({ error: 'Failed to create folder' }, { status: 500 });
    }
}
