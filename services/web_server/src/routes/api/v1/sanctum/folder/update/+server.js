import { ApiServerClient, proxyError } from '$lib/network';
import { json } from '@sveltejs/kit';

export async function POST({ request, locals }) {
    try {
        const payload = await request.json();

        const sessionUser = await locals.session.user.get();
        if (!sessionUser) {
            return json({ status: 401, message: 'Unauthorized' }, { status: 401 });
        }
        payload.user_id = sessionUser.id;

        const response = await ApiServerClient.post('/folder/update', payload, { headers: { 'X-Api-Key': sessionUser.api_key } });

        return json(response.data);
    } catch (error) {
        return proxyError(error, 'Folder update proxy error:', 'Internal Server Error');
    }
}
