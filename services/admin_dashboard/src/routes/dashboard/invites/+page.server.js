import { ApiServerClient, ApiServerError } from '$lib/network';
import { fail } from '@sveltejs/kit';

export const load = async () => {
    try {
        const res = await ApiServerClient.get('/admin/invites');
        return {
            invites: res.data.data.invites
        };
    } catch (err) {
        console.error('Failed to fetch invites:', err);
        return {
            invites: []
        };
    }
};

export const actions = {
    create: async ({ request }) => {
        const data = await request.formData();
        const account_type = data.get('account_type');
        const description = data.get('description');
        const benefit = data.get('benefit');

        try {
            const res = await ApiServerClient.post('/admin/invites', {
                account_type,
                description,
                benefit
            });
            return { success: true, invite: res.data.data.invite };
        } catch (err) {
            console.error('Failed to create invite:', err);
            return fail(500, {
                message: 'Failed to create invite code'
            });
        }
    },
    delete: async ({ request }) => {
        const data = await request.formData();
        const code = data.get('code');

        try {
            await ApiServerClient.delete('/admin/invites/' + code);
            return { success: true };
        } catch (err) {
            console.error('Failed to delete invite:', err);
            return fail(500, {
                message: 'Failed to delete invite code'
            });
        }
    }
};
