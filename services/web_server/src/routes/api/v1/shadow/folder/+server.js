import { json } from '@sveltejs/kit';
import { ApiServerClient, ApiServerRoutes, proxyError } from '$lib/network.js';
import { validateShadowRequest } from '$lib/server/shadow.js';

export async function POST({ request, locals }) {
    let sessionUser = await locals.session.user.get();

    // For SHADOW route, we allow anonymous if API Key is present
    let apiKey = request.headers.get('x-api-key');

    let body;
    try {
        body = await request.json();
        if (body.api_key) apiKey = body.api_key; // Allow body override
    } catch {
        return json({ error: 'Invalid JSON body' }, { status: 400 });
    }

    if (!sessionUser && !apiKey) {
        return json({ error: 'Unauthorized: Session or API Key required' }, { status: 401 });
    }

    // An anonymous key has to exist in `anonymous_users` before api_switch can
    // resolve it, and the drop page creates its batch folder before any file, so
    // this is the first call a brand new visitor makes. Without it the folder
    // 401s, the upload degrades to loose files, and the visitor is shown no
    // share link at all. Registered callers skip it: they already have identity.
    if (!sessionUser) {
        const { success, error } = await validateShadowRequest({ request });
        if (!success) {
            return json({ error: error.message || 'Unauthorized' }, { status: error.status || 401 });
        }
    }

    try {
        let payload = {
            ...body
        };

        if (sessionUser) {
            payload.user_id = sessionUser.id;
        }

        if (apiKey) {
            payload.owner_api_key = apiKey;
        } else if (!sessionUser) {
        }

        let response = await ApiServerClient.post(ApiServerRoutes.createFolder, payload, { headers: { 'X-Api-Key': sessionUser?.api_key || apiKey || undefined } }).then(res => res.data);
        return json(response);
    } catch (err) {
        return proxyError(err, '[CREATE_FOLDER]', 'Failed to create folder');
    }
}
