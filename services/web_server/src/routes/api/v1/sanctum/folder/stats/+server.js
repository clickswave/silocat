import { ApiServerClient } from '$lib/network.js';
import { json } from '@sveltejs/kit';

// POST /api/v1/sanctum/folder/stats  - how many items a folder delete would take.
//
// The delete-folder confirmation asks for this so it can say "N items" instead of
// asking the user to confirm an unknown blast radius. The backend handler and its
// route have existed since folder_stats.rs landed; this proxy did not, so every
// call 404'd on the SvelteKit side and the dialog fell back to "unknown".
export async function POST({ request, locals }) {
	try {
		const user = await locals.session.user.get();
		if (!user) {
			return json({ error: 'Unauthorized' }, { status: 401 });
		}

		const payload = await request.json();

		const res = await ApiServerClient.post(
			'/folder/stats',
			{ ...payload, user_id: user.id },
			{ headers: { 'X-Api-Key': user.api_key } }
		);

		if (res.data.status === 200) {
			return json({
				start: Date.now(),
				success: { status: 200, data: res.data.data }
			});
		}

		return json(
			{
				status: res.data.status || 500,
				message: res.data.message || 'Failed to calculate folder stats',
				errors: [res.data.message],
				data: {}
			},
			{ status: res.data.status || 500 }
		);
	} catch (err) {
		console.error('[FOLDER_STATS_PROXY]', err?.response?.status || err.message);
		return json({ error: 'Failed to calculate folder stats' }, { status: 500 });
	}
}
