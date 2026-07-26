import { json } from '@sveltejs/kit';
import { ApiServerClient } from '$lib/network.js';

export async function POST({ request, locals }) {
    let sessionUser = await locals.session.user.get();
    if (!sessionUser) return json({ error: 'Unauthorized' }, { status: 401 });

    const body = await request.json();
    const { folder_id } = body;

    if (!folder_id) {
        return json({ error: 'Missing folder_id' }, { status: 400 });
    }

    try {
        let payload = {
            user_id: sessionUser.id,
            api_key: sessionUser.api_key,
            folder_id
        };

        // Note: Using the route we registered in folder/mod.rs: /folder/restore
        // ApiServerClient base URL is likely correct, we append path.
        // Assuming ApiServerRoutes.listFolders maps to /folder/list, we can guess.
        // Based on file restore proxy: ApiServerClient.post('/file/restore-files', payload);
        // My folder routes are under /folder/* in mod.rs?
        // No, folder/mod.rs routes are like "/restore", "/permanent-delete".
        // AND folder/mod.rs is likely mounted under "/folder"?
        // Typically api_switch mounts file router under /file and folder router under /folder ??
        // But referencing file/list_files, it was mounted at /list-files inside file/mod.rs?
        // AND file/mod.rs likely mounted at root or `/file`?
        // In `file/list_files.rs`, the route was `/list-files`. 
        // If file/mod.rs is mounted at `/`, then it is `/list-files`.
        // If file/mod.rs is mounted at `/file`, then it is `/file/list-files`.
        // My file restore proxy used `/file/restore-files`. 
        // My file/mod.rs registered `/restore-files`.
        // So `api_switch` MUST mount `file/mod.rs` under `/file`.

        // NOW FOLDER:
        // `folder/mod.rs` registers `/restore`.
        // If `folder/mod.rs` is mounted under `/folder`, then path is `/folder/restore`.
        // Just like `file/mod.rs` registered `/restore-files` and mounted under `/file`.

        // `folder/mod.rs` registered `/restore`.
        // So path is likely `/folder/restore`.

        let response = await ApiServerClient.post('/folder/restore', payload, { headers: { 'X-Api-Key': sessionUser.api_key } });
        return json(response.data);
    } catch (err) {
        console.error('[RESTORE_FOLDER]', err);
        return json({ error: 'Failed to restore folder' }, { status: 500 });
    }
}
