import { ApiServerClient } from '$lib/network';
import { json } from '@sveltejs/kit';

export async function GET({ locals }) {
    try {
        if (!locals.session) return json({ shadow: {}, sanctum: {} }, { status: 401 });
        const res = await ApiServerClient.get('/admin/cloudflare', {
            headers: { Authorization: `Bearer ${locals.session}` }
        });
        return json(res.data.data);
    } catch (err) {
        console.error(err);
        return json({ shadow: {}, sanctum: {} }, { status: 500 });
    }
}
