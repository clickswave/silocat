import { redirect } from '@sveltejs/kit';

export const POST = async (event) => {
    await event.locals.session.delete();
    throw redirect(303, '/login');
};
