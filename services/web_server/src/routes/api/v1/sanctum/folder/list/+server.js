
import { json } from '@sveltejs/kit';
import { ApiServerClient, ApiServerRoutes } from '$lib/network.js';

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
    // However, let's be explicit if we want validation, but ...body is fine for now as payload matches.
    // Actually, let's just leave it as spread ...body, assuming frontend sends { shared: true }
    // But wait, I need to check if I need to do anything special. 
    // The previous implementation of this file just spreads body.
    // So if I send { shared: true } from frontend client, it goes into payload.
    // So actually NO CHANGE needed here if I just send it in body?
    // Let's verify.
    // `body = await request.json()` -> `payload = { ...body, user_id }`
    // Backend expects `shared: Option<bool>`.
    // So `payload` having `shared: true` works.

    // I will just add a comment to confirm clarity or logic if needed.
    // Actually, I'll essentially do nothing but since I am here, maybe I should just explicitly log or ensuring it works?
    // No, cleaner code is better. I can skip this file edit if it already supports generic body forwarding.
    // Looking at line 19: `...body`. Yes.

    // Correction: The plan said "Update frontend proxies".
    // I should ensure consistency. 
    // The file list proxy uses GET params.
    // The folder list proxy uses POST body.

    // So for file list (GET), I added query param handling.
    // For folder list (POST), it's already generic.
    // I'll skip editing this file.

    try {
        let payload = {
            ...body,
            user_id: sessionUser.id
        };

        let response = await ApiServerClient.post(ApiServerRoutes.listFolders, payload).then(res => res.data);
        return json(response);
    } catch (err) {
        console.error('[FETCH_FOLDERS]', err);
        return json({ error: 'Failed to fetch folders' }, { status: 500 });
    }
}
