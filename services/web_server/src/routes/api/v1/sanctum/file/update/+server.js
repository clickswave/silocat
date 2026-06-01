import { ApiServerClient } from '$lib/network';
import { json } from '@sveltejs/kit';

export async function POST({ request, locals }) {
    try {
        const payload = await request.json();
        // Inject user_id from session if needed, but backend uses user_id from payload?
        // Wait, api_switch update_files.rs uses payload.user_id.
        // Usually validation middleware assumes user_id.
        // Let's verify if ApiServerClient sends headers that allow validation.
        // Yes, ApiServerClient usually sends auth headers.
        // But payload.user_id must match.
        // Let's assume we need to inject user_id into payload or backend validates token anyway.
        // Let's send user_id from session to be safe.
        const sessionUser = await locals.session.user.get();
        if (!sessionUser) {
            return json({ status: 401, message: 'Unauthorized' }, { status: 401 });
        }
        const userId = sessionUser.id;
        payload.user_id = userId;

        const response = await ApiServerClient.post('/file/update-files', payload);

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
