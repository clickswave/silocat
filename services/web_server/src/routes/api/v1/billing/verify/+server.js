
import { json } from '@sveltejs/kit';
import { ApiServerClient } from '$lib/network.js';

export async function POST({ request, locals }) {
    let user = await locals.session.user.get();
    if (!user) {
        return json({ error: 'Unauthorized' }, { status: 401 });
    }

    try {
        const payload = await request.json();
        // /billing sits behind validate_token, so the caller's key has to travel
        // with the request. Without it the gateway takes the money and the grant
        // is refused, which is the worst possible way for this call to fail.
        let response = await ApiServerClient.post('/billing/verify', payload, {
            headers: { 'X-Api-Key': user.api_key }
        });

        // Fetch fresh user data to update session
        if (response.status === 200) {
            const userRes = await ApiServerClient.get('/user/info', {
                headers: {
                    'X-Api-Key': user.api_key
                }
            });
            const updatedUser = userRes.data.data;
            await locals.session.user.set(updatedUser);
        }

        return json({ success: response.data });
    } catch (err) {
        console.error('[POST_BILLING_VERIFY]', err);
        return json({ error: 'Failed to verify payment' }, { status: 500 });
    }
}
