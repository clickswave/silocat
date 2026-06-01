import { json } from '@sveltejs/kit';
import { ApiServerClient, ApiServerError } from '$lib/network';

export async function POST({ request, locals }) {
    const user = await locals.session.user.get();

    if (!user) {
        return json({ status: 401, message: 'Unauthorized', data: {} });
    }

    try {
        const response = await ApiServerClient.get('/admin/early-access');
        console.log('Early Access API Response:', response.status, response.data);
        return json(response.data);
    } catch (error) {
        console.error('Error fetching early access requests:', error.response?.status, error.response?.data || error.message);
        return json(ApiServerError);
    }
}
