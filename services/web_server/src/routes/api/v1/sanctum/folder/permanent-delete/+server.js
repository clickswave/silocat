import { json } from '@sveltejs/kit';
import { ApiServerClient } from '$lib/network.js';

export async function POST({ request, locals }) {
    let sessionUser = await locals.session.user.get();
    if (!sessionUser) return json({ error: 'Unauthorized' }, { status: 401 });

    const body = await request.json();
    const { folder_id } = body;

    if (!folder_id) {
        return json({ error: 'Missing folder_id' }, { status: 400 });
    }

    try {
        let payload = {
            user_id: sessionUser.id,
            api_key: sessionUser.api_key,
            folder_id
        };

        let response = await ApiServerClient.post('/folder/permanent-delete', payload, { headers: { 'X-Api-Key': sessionUser.api_key } });
        return json(response.data);
    } catch (err) {
        console.error('[PERMANENT_DELETE_FOLDER]', err);
        return json({ error: 'Failed to delete folder permanently' }, { status: 500 });
    }
}
