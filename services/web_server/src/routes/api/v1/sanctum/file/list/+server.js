
import { json } from '@sveltejs/kit';
import { ApiServerClient, ApiServerRoutes } from '$lib/network.js';

export async function GET({ url, locals }) {
    let user = await locals.session.user.get();
    if (!user) {
        return json({ error: 'Unauthorized' }, { status: 401 });
    }

    try {
        const folderId = url.searchParams.get('folder_id') || null;
        const starred = url.searchParams.get('starred') === 'true'; // Convert string to boolean

        // Construct payload. Note: backend list_files expects folder_id: Option<String>, starred: Option<bool>
        // If starred is false/null, we might as well omit it or pass null/false.
        // Backend logic: if payload.starred == Some(true) -> filter by starred.

        let payload = {
            user_id: user.id,
            folder_id: folderId
        };

        if (starred) {
            payload.starred = true;
        }

        const shared = url.searchParams.get('shared') === 'true';
        if (shared) {
            payload.shared = true;
        }

        let response = await ApiServerClient.post(ApiServerRoutes.listFiles, payload);

        return json(response.data);
    } catch (err) {
        console.error('[GET_SANCTUM_FILES]', err);
        return json({ error: 'Failed to fetch files' }, { status: 500 });
    }
}

export async function POST({ request, locals }) {
    let user = await locals.session.user.get();
    if (!user) {
        return json({ error: 'Unauthorized' }, { status: 401 });
    }

    let body = {};
    try {
        body = await request.json().catch(() => ({}));
    } catch (e) {
        // ignore
    }

    try {
        let payload = {
            ...body,
            user_id: user.id
        };

        let response = await ApiServerClient.post(ApiServerRoutes.listFiles, payload);

        return json(response.data);
    } catch (err) {
        console.error('[POST_SANCTUM_FILES]', err);
        return json({ error: 'Failed to fetch files' }, { status: 500 });
    }
}
