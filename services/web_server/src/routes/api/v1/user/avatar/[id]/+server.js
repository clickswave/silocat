import { ApiServerClient } from '$lib/network.js';

// GET /api/v1/user/avatar/:id  - redirect to a fresh presigned R2 URL.
// The browser caches a stable <img src>; each load 302s to a short-lived URL,
// so the bucket stays private and links never go stale. ?v= busts the cache.
export async function GET({ params }) {
	try {
		const res = await ApiServerClient.get('/user/avatar-url', {
			params: { user_id: params.id }
		});
		const url = res.data?.data?.url;
		if (!url) {
			return new Response('Not found', { status: 404 });
		}
		return new Response(null, {
			status: 302,
			headers: { Location: url, 'Cache-Control': 'no-store' }
		});
	} catch {
		return new Response('Not found', { status: 404 });
	}
}
