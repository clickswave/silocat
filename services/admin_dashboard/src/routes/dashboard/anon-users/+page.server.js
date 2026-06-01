import { ApiServerClient } from '$lib/network';

export const load = async ({ locals }) => {
    try {
        if (!locals.session) return {};
        const res = await ApiServerClient.get('/admin/anon-users');
        return {
            users: res.data.data.users || []
        };
    } catch (err) {
        console.error(err);
        return { users: [] };
    }
};
