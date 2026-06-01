import { fail } from '@sveltejs/kit';
import { ApiServerClient, ApiServerRoutes, ApiServerError } from '$lib/network';
import { validateTurnstileToken } from '$lib/turnstile.js';
import { env } from '$env/dynamic/public';

const { PUBLIC_TURNSTILE_KEY } = env;

export const load = async () => {
    return {
        turnstileSiteKey: PUBLIC_TURNSTILE_KEY
    };
};

export const actions = {
    default: async ({ request }) => {
        const data = await request.formData();
        const email = data.get('email');
        const turnstileToken = data.get('cf-turnstile-response');

        if (!email) {
            return fail(400, { email, missing: true });
        }

        // Validate Turnstile
        const { success: turnstileSuccess } = await validateTurnstileToken(turnstileToken);
        if (!turnstileSuccess) {
            return fail(400, {
                email,
                error: 'Security check failed. Please try again.'
            });
        }

        // Basic email validation
        const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
        if (!emailRegex.test(email)) {
            return fail(400, { email, invalid: true });
        }

        try {
            await ApiServerClient.post(ApiServerRoutes.earlyAccess, { email });
        } catch (error) {
            console.error('Early Access Error:', error.response?.data || error);

            if (error.response?.status === 409) {
                return fail(400, { email, error: 'You have already requested early access.' });
            }

            return fail(500, {
                email,
                error: error.response?.data?.message || 'Registration failed. Please try again later.'
            });
        }

        return { success: true };
    }
};
