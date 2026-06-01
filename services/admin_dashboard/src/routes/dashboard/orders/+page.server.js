import { ApiServerClient } from '$lib/network';

export const load = async ({ locals }) => {
    try {
        if (!locals.session) return {};
        const res = await ApiServerClient.get('/admin/orders');
        return {
            orders: res.data.data.orders || []
        };
    } catch (err) {
        console.error(err);
        return { orders: [] };
    }
};
