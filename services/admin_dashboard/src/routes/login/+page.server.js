import { fail, redirect } from '@sveltejs/kit';
import { ApiServerClient, ApiServerError } from '$lib/network';

/** @satisfies {import('./$types').Load} */
export const load = async ({ locals }) => {
    let session = await locals.session.get();

    if (session) throw redirect(302, '/dashboard');
    return {
        session,
    };
};

export const actions = {
    default: async (event) => {
        const data = await event.request.formData();
        const email = data.get('email');
        const password = data.get('password');

        if (!email || !password) {
            return fail(400, { email, missing: true });
        }

        try {
            const res = await ApiServerClient.post('/admin/login', { email, password });

            if (res.status === 200) {
                const adminData = res.data.data.admin;

                // Set session
                await event.locals.session.user.set(adminData);

                throw redirect(303, '/dashboard');
            } else {
                return fail(401, { email, incorrect: true, message: res.data.message });
            }
        } catch (err) {
            if (err.status === 303 && err.location) {
                throw err;
            }

            console.error('Login error:', err);

            if (err.response) {
                return fail(err.response.status, {
                    email,
                    error: true,
                    message: err.response.data?.message || 'Login failed'
                });
            }

            return fail(500, {
                email,
                error: true,
                message: ApiServerError.message
            });
        }
    }
};
