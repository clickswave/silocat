import { redirect } from '@sveltejs/kit';

export const load = async (event) => {
    const user = await event.locals.session.user.get();

    if (!user) {
        throw redirect(303, '/login');
    }

    return {
        user
    };
};
