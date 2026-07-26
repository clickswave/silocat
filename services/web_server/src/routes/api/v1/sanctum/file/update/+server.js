import { ApiServerClient } from '$lib/network';
import { json } from '@sveltejs/kit';

export async function POST({ request, locals }) {
    try {
        const payload = await request.json();
        // Inject user_id from session if needed, but backend uses user_id from payload?
        // Usually validation middleware assumes user_id.
        // Yes, ApiServerClient usually sends auth headers.
        // But payload.user_id must match.
        const sessionUser = await locals.session.user.get();
        if (!sessionUser) {
            return json({ status: 401, message: 'Unauthorized' }, { status: 401 });
        }
        const userId = sessionUser.id;
        payload.user_id = userId;

        const response = await ApiServerClient.post('/file/update-files', payload, { headers: { 'X-Api-Key': sessionUser.api_key } });

        return json(response.data);
    } catch (error) {
        console.error('File update proxy error:', error);
        return json(
            {
                status: 500,
                message: 'Internal Server Error'
            },
            { status: 500 }
        );
    }
}
