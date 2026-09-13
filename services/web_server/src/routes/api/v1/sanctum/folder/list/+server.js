
import { json } from '@sveltejs/kit';
import { ApiServerClient, ApiServerRoutes, proxyError } from '$lib/network.js';

export async function POST({ request, locals }) {
    let sessionUser = await locals.session.user.get();
    if (!sessionUser) return json({ error: 'Unauthorized' }, { status: 401 });

    let body = {};
    try {
        // Optional body for parent_id
        body = await request.json().catch(() => ({}));
    } catch (e) {
        // ignore
    }

    // Ensure we capture shared flag from body if present
    // The previous code spreads ...body so it should already be included if passed.
    // The previous implementation of this file just spreads body.
    // So if I send { shared: true } from frontend client, it goes into payload.
    // So actually NO CHANGE needed here if I just send it in body?
    // `body = await request.json()` -> `payload = { ...body, user_id }`
    // Backend expects `shared: Option<bool>`.
    // So `payload` having `shared: true` works.

    // No, cleaner code is better. I can skip this file edit if it already supports generic body forwarding.
    // Looking at line 19: `...body`. Yes.

    // Correction: The plan said "Update frontend proxies".
    // I should ensure consistency. 
    // The file list proxy uses GET params.
    // The folder list proxy uses POST body.

    // So for file list (GET), I added query param handling.
    // For folder list (POST), it's already generic.

    try {
        let payload = {
            ...body,
            user_id: sessionUser.id
        };

        let response = await ApiServerClient.post(ApiServerRoutes.listFolders, payload, { headers: { 'X-Api-Key': sessionUser.api_key } }).then(res => res.data);
        return json(response);
    } catch (err) {
        return proxyError(err, '[FETCH_FOLDERS]', 'Failed to fetch folders');
    }
}
