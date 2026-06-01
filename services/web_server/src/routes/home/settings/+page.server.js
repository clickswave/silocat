
import { fail } from '@sveltejs/kit';
import { ApiServerClient } from '$lib/network';

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
                bio
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
