import { ApiServerClient } from '$lib/network.js';
import { json } from '@sveltejs/kit';

export async function POST({ request, locals }) {
    try {
        const user = await locals.session.user.get();
        if (!user) {
            return json({ error: 'Unauthorized' }, { status: 401 });
        }

        const payload = await request.json();

        console.log('[STAR_FOLDER_PROXY] User ID:', user.id);

        const res = await ApiServerClient.post('/file/star/folder', {
            ...payload,
            user_id: user.id
        }, { headers: { 'X-Api-Key': user.api_key } });

        console.log('[STAR_FOLDER_PROXY] Backend Status:', res.data?.status);
        console.log('[STAR_FOLDER_PROXY] Backend Message:', res.data?.message);


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
        console.error('[STAR_FOLDER_PROXY_ERROR]', err);
        return json({ error: err.message || 'Internal Server Error' }, { status: 500 });
    }
}
