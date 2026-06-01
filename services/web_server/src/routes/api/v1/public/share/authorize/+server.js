
import { json } from '@sveltejs/kit';
import { ApiServerClient } from '$lib/network.js';

export async function POST({ request }) {
    try {
        const payload = await request.json();

        // Public endpoint, no auth needed
        const res = await ApiServerClient.post('/file/public/share/authorize', payload);

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
        console.error('[PUBLIC_AUTH_PROXY]', err);
        return json({ error: 'Internal Error' }, { status: 500 });
    }
}
