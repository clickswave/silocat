import { ApiServerClient } from '$lib/network';
import { json } from '@sveltejs/kit';

export const GET = async ({ locals }) => {
    try {
        if (!locals.session) return json({ error: 'Unauthorized' }, { status: 401 });
        const res = await ApiServerClient.get('/admin/orders');
        return json({ orders: res.data.data.orders || [] });
    } catch (err) {
        console.error('Failed to fetch orders:', err);
        return json({ error: 'Failed' }, { status: 500 });
    }
};
