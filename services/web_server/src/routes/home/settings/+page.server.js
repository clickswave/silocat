
import { fail } from '@sveltejs/kit';
import { ApiServerClient } from '$lib/network';

export const load = async ({ locals }) => {
	const sessionUser = await locals.session.user.get();
	if (!sessionUser) return { usernameStatus: null };
	try {
		const res = await ApiServerClient.get('/user/username-status', {
			headers: { 'X-Api-Key': sessionUser.api_key }
		});
		return { usernameStatus: res.data?.data ?? null };
	} catch (err) {
		console.error('username-status load error:', err?.response?.data || err.message);
		return { usernameStatus: null };
	}
};

export const actions = {
    saveProfile: async ({ request, locals }) => {
        const sessionUser = await locals.session.user.get();
        if (!sessionUser) {
            return fail(401, { message: 'Unauthorized' });
        }

        const formData = await request.formData();
        const username = formData.get('username');
        const country = formData.get('country') || null;
        const bio = formData.get('bio') || null;

        try {
            const response = await ApiServerClient.post('/user/update-profile', {
                country,
                bio,
                username
            }, {
                headers: { 'X-Api-Key': sessionUser.api_key }
            });

            if (response.data && response.data.data) {
                await locals.session.user.set(response.data.data);
            }

            return { success: true };
        } catch (err) {
            console.error('Update profile error:', err);
            const errorMessage = err.response?.data?.message || err.response?.data?.error || 'Failed to update profile';
            return fail(err.response?.status || 500, {
                message: errorMessage,
                username,
                country: country || ''
            });
        }
    }
};
