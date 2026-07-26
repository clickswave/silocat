import { redirect } from '@sveltejs/kit';

/**
 * `/home/subscription` and `/home/billing` used to be two competing plan
 * pickers. They are one page now; this keeps old links and bookmarks working.
 */
export function load() {
	redirect(308, '/home/billing');
}
