import { env } from '$env/dynamic/private';
const { TURNSTILE_SECRET } = env;

export async function validateTurnstileToken(token) {
	const response = await fetch(
		'https://challenges.cloudflare.com/turnstile/v0/siteverify',
		{
			method: 'POST',
			headers: {
				'content-type': 'application/json',
			},
			body: JSON.stringify({
				response: token,
				secret: TURNSTILE_SECRET,
			}),
		},
	);

	const data = await response.json();

	return {
		// Return the status
		success: data.success,

		// Return the first error if it exists
		error: data['error-codes']?.length ? data['error-codes'][0] : null,
	};
}
