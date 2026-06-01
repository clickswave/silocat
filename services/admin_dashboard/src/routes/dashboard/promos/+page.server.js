import { ApiServerClient } from '$lib/network';
import { fail } from '@sveltejs/kit';

export const load = async ({ locals }) => {
    try {
        if (!locals.session) return {}; // Protected by layout
        const res = await ApiServerClient.get('/admin/promos');
        return {
            promos: res.data.data.promos || []
        };
    } catch (err) {
        console.error(err);
        return { promos: [] };
    }
};

export const actions = {
    create: async ({ request }) => {
        const data = await request.formData();
        const code = data.get('code');
        const discount = data.get('discount');
        const duration = data.get('duration');
        const active = true;

        if (!code || !discount || !duration) {
            return fail(400, { missing: true });
        }

        try {
            await ApiServerClient.post('/admin/promos', {
                code,
                discount_percentage: parseInt(discount),
                duration,
                active
            });
            return { success: true };
        } catch (err) {
            console.error(err);
            return fail(500, { error: 'Failed to create promo' });
        }
    }
};
