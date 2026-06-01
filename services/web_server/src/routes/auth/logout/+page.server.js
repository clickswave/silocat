import { redirect } from '@sveltejs/kit';

/** @satisfies {import('./$types').Actions} */
export const actions = {
	default: async ({ cookies }) => {
		cookies.delete('silocat-session', { path: '/' });
		throw redirect(303, '/');
	}
};
