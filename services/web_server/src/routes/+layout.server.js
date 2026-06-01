import { redirect } from '@sveltejs/kit';

export const load = async ({ locals, url }) => {
	try {
		let session = await locals.session.get();
		let user = session?.user;
		let storage = session?.storage;

		let totalAvailableSpace = Number(user?.default_storage_bytes) || 0;
		if (user?.subscription?.additional_space) {
			totalAvailableSpace += Number(user.subscription.additional_space);
		}

		if (user) {
			user.totalAvailableSpace = totalAvailableSpace;

			// Enforce email verification
			if (!user.email_verified) {
				if (!url.pathname.startsWith('/home/pending-actions') && !url.pathname.startsWith('/auth')) {
					return redirect(302, '/home/pending-actions');
				}
			} else {
				if (url.pathname.startsWith('/home/pending-actions')) {
					return redirect(302, '/home');
				}
			}
		}

		console.log({ "auth": { user, storage, subscription: user?.subscription } });

		return {
			user,
			storage
		};
	} catch (e) {
		if (e.status && e.location) {
			throw e;
		}
		console.error("Layout load error:", e);
		return { user: null, storage: null };
	}
};
