import { json } from '@sveltejs/kit';
import { ApiServerClient } from '$lib/network.js';

export async function POST({ request }) {
    try {
        const payload = await request.json();

        // Forward to backend
        const res = await ApiServerClient.post('/file/public/share/fetch-chunks', payload);

        // Normalize response to match expected frontend structure
        if (res.data.success) {
            return json({ success: res.data.success });
        } else if (res.data.status === 200) {
            // Some backend endpoints return different structures
            return json({ success: { data: res.data.data } });
        }

        // Fallback for direct proxy if backend structure is already correct
        return json(res.data);

    } catch (e) {
        console.error('[FETCH_CHUNKS_PROXY]', e);
        return json({ error: 'Internal Error' }, { status: 500 });
    }
}
