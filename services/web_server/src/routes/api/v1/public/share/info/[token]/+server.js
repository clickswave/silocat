
import { json } from '@sveltejs/kit';
import { ApiServerClient } from '$lib/network.js';

export async function GET({ params }) {
    try {
        const { token } = params;

        // Public endpoint, no auth needed
        const res = await ApiServerClient.get(`/file/public/share/info/${token}`)
            .then(r => r.data);

        if (res.status === 200) {
            return json({
                start: Date.now(),
                success: {
                    status: 200,
                    data: res.data
                }
            });
        } else if (res.status === 410) {
            return json({ error: 'Link Expired', code: 'LINK_EXPIRED' }, { status: 410 });
        } else if (res.status === 404) {
            return json({ error: 'Not Found', code: 'NOT_FOUND' }, { status: 404 });
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
        // api_switch maps its envelope status onto the HTTP code, so anything
        // that is not a 2xx makes axios throw and lands here. That made the
        // 410/404 branches above dead code and turned "this link has expired"
        // into a server error, which is the message the /s/<token> page then
        // could not show. Carry the backend's status and body through.
        const upstream = err?.response?.data;
        if (upstream) return json(upstream, { status: err.response.status });
        console.error('[PUBLIC_INFO_PROXY]', err);
        return json({ error: 'Internal Error' }, { status: 500 });
    }
}
