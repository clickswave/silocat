import { ApiServerClient } from '$lib/network';

export const load = async ({ locals }) => {
    try {
        if (!locals.session) return {}; // Protected by layout
        const res = await ApiServerClient.get('/admin/users');
        return {
            users: res.data.data.users || []
        };
    } catch (err) {
        console.error(err);
        return { users: [] };
    }
};

export const actions = {
    delete: async ({ request, locals }) => {
        if (!locals.session) return { success: false, error: 'Unauthorized' };

        const data = await request.formData();
        const id = data.get('id');

        try {
            await ApiServerClient.delete(`/admin/users/${id}`, {
                headers: { Authorization: `Bearer ${locals.session}` }
            });
            return { success: true };
        } catch (err) {
            console.error('Delete User Error:', err);
            return { success: false, error: err.message };
        }
    }
};
