import {
	ApiServerClient,
	ApiServerRoutes,
	ApiServerError
} from '$lib/network';
import { redirect } from '@sveltejs/kit';
import { validateTurnstileToken } from '$lib/turnstile.js';
import { env } from '$env/dynamic/public';
const { PUBLIC_TURNSTILE_KEY } = env;

/** @satisfies {import('./$types').Load} */
export const load = async ({ locals }) => {
	let session = await locals.session.get();

	console.log({ session });

	if (session) throw redirect(302, '/home');

	return {
		session,
		turnstileSiteKey: PUBLIC_TURNSTILE_KEY };
};

/** @satisfies {import('./$types').Actions} */
export const actions = {

	default: async ({ request, locals }) => {
		let data = await request.formData();

		let username = data.get('username')?.trim();
		let email = data.get('email')?.trim();
		let password = data.get('password')?.trim();
		let inviteCode = data.get('inviteCode')?.trim();

		let turnstile_token = data.get('cf-turnstile-response');
		let { success } = await validateTurnstileToken(turnstile_token);

		console.log({SIGNUP_RESPONSE: {data, success}});
		if (!success) {
			return {
				error: {
					status: 400, message: 'Could not validate captcha', errors: ['Cloudflare says you\'re not a human!'], data: {}
				}
			};
		}

		try {
			let payload = {
				username: username,
				email: email,
				password: password,
				invite_code: inviteCode || null
			};
			let response = await ApiServerClient.post(ApiServerRoutes.registerPersonal, payload).then((res) => res.data);
			await locals.session.user.set(response.data.user);
			console.log('[*] User logged in successfully');

			return {
				success: response
			};

		} catch (e) {
			console.error('[*] Error when signing up:', e);
			if (e.response) {
				console.error('Response data:', e.response.data);
				console.error('Response status:', e.response.status);
			}
			return { error: e.response?.data || { message: e.message || 'Unknown error occurred' } };
		}
	}
};
