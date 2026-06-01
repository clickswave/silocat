import { ApiServerClient } from '$lib/network';
import { json } from '@sveltejs/kit';

export const GET = async ({ locals }) => {
    try {
        // Verify session exists (though layout protects this usually)
        if (!locals.session) {
            return json({ error: 'Unauthorized' }, { status: 401 });
        }

        const res = await ApiServerClient.get('/admin/invites');
        return json(res.data.data); // data.invites is inside here
    } catch (err) {
        console.error('Failed to fetch invites API:', err);
        return json({ error: 'Failed to fetch invites' }, { status: 500 });
    }
};
