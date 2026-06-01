import { ApiServerClient } from '$lib/network';
import { json } from '@sveltejs/kit';

export async function POST({ request, locals }) {
    try {
        const payload = await request.json();

        const sessionUser = await locals.session.user.get();
        if (!sessionUser) {
            return json({ status: 401, message: 'Unauthorized' }, { status: 401 });
        }
        payload.user_id = sessionUser.id;

        const response = await ApiServerClient.post('/folder/update', payload);

        return json(response.data);
    } catch (error) {
        console.error('Folder update proxy error:', error);
        return json(
            {
                status: 500,
                message: 'Internal Server Error'
            },
            { status: 500 }
        );
    }
}
