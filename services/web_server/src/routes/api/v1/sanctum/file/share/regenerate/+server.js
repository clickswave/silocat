
import { json } from '@sveltejs/kit';
import { ApiServerClient } from '$lib/network.js';

export async function POST({ request, locals }) {
    try {
        const user = await locals.session.user.get();
        if (!user) {
            return json({ error: 'Unauthorized' }, { status: 401 });
        }

        const payload = await request.json();

        const res = await ApiServerClient.post('/file/share/regenerate', {
            ...payload,
            user_id: user.id
        }, { headers: { 'X-Api-Key': user.api_key } });

        if (res.data.status === 200) {
            return json({
                start: Date.now(),
                success: {
                    status: 200,
                    data: res.data.data
                }
            });
        }

        return json(
            {
                status: res.data.status || 500,
                message: res.data.message || 'Failed',
                errors: [res.data.message],
                data: {}
            },
            { status: res.data.status || 500 }
        );

    } catch (err) {
        console.error('[SHARE_REGEN_PROXY]', err);
        return json({ error: 'Internal Error' }, { status: 500 });
    }
}
