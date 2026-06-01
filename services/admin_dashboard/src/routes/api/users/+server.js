import { ApiServerClient } from '$lib/network';
import { json } from '@sveltejs/kit';

export const GET = async ({ locals }) => {
    try {
        if (!locals.session) return json({ error: 'Unauthorized' }, { status: 401 });
        const res = await ApiServerClient.get('/admin/users');
        return json({ users: res.data.data.users || [] });
    } catch (err) {
        console.error('Failed to fetch users:', err);
        return json({ error: 'Failed' }, { status: 500 });
    }
};
