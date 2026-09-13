import { json } from '@sveltejs/kit';
import { ApiServerClient, proxyError } from '$lib/network.js';

export async function POST({ request, locals }) {
    let sessionUser = await locals.session.user.get();
    if (!sessionUser) return json({ error: 'Unauthorized' }, { status: 401 });

    const body = await request.json();
    const { file_id } = body;

    if (!file_id) {
        return json({ error: 'Missing file_id' }, { status: 400 });
    }

    try {
        let payload = {
            user_id: sessionUser.id,
            api_key: sessionUser.api_key,
            file_id
        };

        let response = await ApiServerClient.post('/file/restore-files', payload, { headers: { 'X-Api-Key': sessionUser.api_key } });
        return json(response.data);
    } catch (err) {
        return proxyError(err, '[RESTORE_FILE]', 'Failed to restore file');
    }
}
