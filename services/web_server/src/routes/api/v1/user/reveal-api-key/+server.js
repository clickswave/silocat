import { json } from '@sveltejs/kit';

/**
 * POST /api/v1/user/reveal-api-key
 *
 * Hands the caller their own API key, on demand.
 *
 * The root layout strips `api_key` from the `user` it returns, because a load()
 * return value lands in the page hydration payload where any client script,
 * extension or error reporter can read it. That is the right call and it stays.
 * But the Settings page read `data.user?.api_key` to fill its key field, so the
 * field was permanently empty: the masked value was blank, the eye toggled
 * between two empty strings, and Copy did nothing. The only way to obtain a key
 * was to press Rotate, which invalidates the key your integrations are using.
 *
 * So the key is fetched on demand instead of being shipped with the page. It
 * crosses the wire only when the user asks to see it, never sits in the HTML,
 * and is not in the payload of a page they merely navigated past.
 *
 * Session-only, like rotate: a caller holding a stolen key must not be able to
 * use it to read the key back out, and key-authenticated callers already have it.
 */
export async function POST({ locals }) {
	const session = await locals.session.get();
	const user = session?.user;
	if (!session || !user) return json({ error: 'Unauthorized' }, { status: 401 });

	if (!user.api_key) {
		return json({ error: 'No API key on this account' }, { status: 404 });
	}

	return json(
		{ success: { api_key: user.api_key } },
		{ headers: { 'Cache-Control': 'no-store' } }
	);
}
