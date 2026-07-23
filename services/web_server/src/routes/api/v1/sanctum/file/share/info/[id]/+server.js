
import { json } from '@sveltejs/kit';
import { ApiServerClient } from '$lib/network.js';

export async function GET({ params, locals }) {
    try {
        const user = await locals.session.user.get();
        if (!user) {
            return json({ error: 'Unauthorized' }, { status: 401 });
        }

        const { id } = params;

        // Passing user_id as query param to match the pattern of bypassing Extension middleware
        // This requires backend update for `get_share_info` too.
        const res = await ApiServerClient.get(`/file/share/info/${id}?user_id=${user.id}`, { headers: { 'X-Api-Key': user.api_key } })
            .then(r => r.data);

        if (res.status === 200) {
            return json({
                start: Date.now(),
                success: {
                    status: 200,
                    data: res.data
                }
            });
        }

        return json(
            {
                status: res.status || 500,
                message: res.message || 'Failed',
                errors: [res.message],
                data: {}
            },
            { status: res.status || 500 }
        );

    } catch (err) {
        console.error('[SHARE_INFO_PROXY]', err);
        return json({ error: 'Internal Error' }, { status: 500 });
    }
}
