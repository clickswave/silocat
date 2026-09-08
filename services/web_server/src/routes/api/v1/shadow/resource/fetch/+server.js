import { json } from '@sveltejs/kit';
import { ApiServerClient, ApiServerRoutes } from '$lib/network.js';

export async function POST({ request, locals }) {
    let body;
    try {
        body = await request.json();
    } catch {
        return json({ error: 'Invalid JSON body' }, { status: 400 });
    }

    try {
        // Inject caller identity for backend ownership checks (session or shadow key).
        const sessionUser = await locals.session.user.get();
        const api_key = sessionUser?.api_key || request.headers.get('X-Api-Key') || undefined;
        const response = await ApiServerClient.post(
            ApiServerRoutes.fetchResource,
            { ...body, api_key },
            { headers: { 'X-Api-Key': api_key } }
        ).then((res) => res.data);

        // api_switch answers 200 with its own status in the envelope, but axios
        // throws on any non-2xx, so a plain "no such id" used to land in the catch
        // below and reach the browser as a 500. The download page then showed
        // "Failed to load resource" for a link that was merely gone, and its
        // `status === 404` branch could never run. Carry the backend's status.
        const status = response?.status && Number.isInteger(response.status) ? response.status : 200;
        return json(response, { status: status >= 400 ? status : 200 });
    } catch (err) {
        const upstream = err?.response?.data;
        if (upstream) {
            return json(upstream, { status: err.response.status });
        }
        console.error('[FETCH_RESOURCE]', err);
        return json({ error: 'Failed to fetch resource' }, { status: 500 });
    }
}
