import { ApiServerClient, ApiServerRoutes, ApiServerError } from '$lib/network';

import { redirect } from '@sveltejs/kit';
import { validateTurnstileToken } from '$lib/turnstile.js';
import { env } from '$env/dynamic/public';

const { PUBLIC_TURNSTILE_KEY } = env;

/** @satisfies {import('./$types').Load} */
export const load = async ({ locals }) => {
	let session = await locals.session.get();

	if (session) throw redirect(302, '/home');
	return {
		session,
		turnstileSiteKey: PUBLIC_TURNSTILE_KEY
	};
};

/** @satisfies {import('./$types').Actions} */
export const actions = {
	default: async ({ request, locals }) => {
		let data = await request.formData();
		let email = data.get('email')?.trim();
		let password = data.get('password')?.trim();

		// let turnstile_token = data.get('cf-turnstile-response');
		// let { success } = await validateTurnstileToken(turnstile_token);
		// if (!success) {
		// 	return {
		// 		error: {
		// 			status: 400,
		// 			message: 'Could not validate captcha',
		// 			errors: ["Cloudflare says you're not a human!"],
		// 			data: {}
		// 		}
		// 	};
		// }

		try {
			let payload = {
				email: email,
				password: password
			};
			let response = await ApiServerClient.post(ApiServerRoutes.login, payload).then(
				(res) => res.data
			);
			await locals.session.user.set(response.data.user);
			console.log('[*] User logged in successfully');

			return {
				success: response
			};
		} catch (e) {
			console.error('[*] Error when signing up:\n', e.response?.data);
			return { error: e.response?.data || ApiServerError };
		}
	}
};
