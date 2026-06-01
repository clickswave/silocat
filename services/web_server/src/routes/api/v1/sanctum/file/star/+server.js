import { ApiServerClient } from '$lib/network.js';
import { json } from '@sveltejs/kit';

export async function POST({ request, locals }) {
    try {
        const user = await locals.session.user.get();
        if (!user) {
            return json({ error: 'Unauthorized' }, { status: 401 });
        }

        const payload = await request.json();

        console.log('[STAR_PROXY] User ID:', user.id);
        console.log('[STAR_PROXY] Payload:', JSON.stringify(payload));

        // Forward the request to the backend
        const res = await ApiServerClient.post('/file/star/file', {
            ...payload,
            user_id: user.id
        });

        console.log('[STAR_PROXY] Backend Status:', res.data?.status);
        console.log('[STAR_PROXY] Backend Message:', res.data?.message);


        if (res.data.status === 200) {
            return json({
                start: Date.now(),
                success: {
                    status: 200,
                    data: res.data.data
                }
            });
        } else {
            return json(
                {
                    status: res.data.status,
                    message: res.data.message || 'Failed to update star status',
                    errors: [res.data.message],
                    data: {}
                },
                { status: 500 }
            );
        }
    } catch (err) {
        console.error('[STAR_FILE_PROXY_ERROR]', err);
        return json({ error: err.message || 'Internal Server Error' }, { status: 500 });
    }
}
