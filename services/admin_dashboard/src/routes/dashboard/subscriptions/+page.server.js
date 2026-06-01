import { ApiServerClient } from '$lib/network';

export const load = async ({ locals }) => {
    try {
        if (!locals.session) return {};
        const res = await ApiServerClient.get('/admin/subscriptions');
        return {
            subscriptions: res.data.data.subscriptions || []
        };
    } catch (err) {
        console.error(err);
        return { subscriptions: [] };
    }
};
