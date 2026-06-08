import { json } from '@sveltejs/kit';
import { ApiServerClient } from '$lib/network.js';

const MAX_BYTES = 1024 * 1024; // 1 MB

// POST /api/v1/user/avatar  - raw image bytes in the body.
export async function POST({ request, locals }) {
	const user = await locals.session.user.get();
	if (!user) {
		return json({ error: 'Unauthorized' }, { status: 401 });
	}

	const contentType = request.headers.get('content-type') || 'application/octet-stream';
	const buf = Buffer.from(await request.arrayBuffer());

	if (buf.length === 0) {
		return json({ error: 'No image provided' }, { status: 400 });
	}
	if (buf.length > MAX_BYTES) {
		return json({ error: 'Image must be 1 MB or smaller.' }, { status: 413 });
	}

	try {
		const res = await ApiServerClient.post('/user/avatar', buf, {
			headers: { 'X-Api-Key': user.api_key, 'Content-Type': contentType },
			maxBodyLength: Infinity,
			maxContentLength: Infinity
		});

		const profile_image = res.data?.data?.profile_image ?? null;
		if (profile_image) {
			// Reflect the new avatar in the session so the UI updates without re-login.
			await locals.session.user.update({ key: 'profile_image', value: profile_image });
		}
		return json({ success: res.data, profile_image });
	} catch (err) {
		const status = err?.response?.status || 500;
		const message = err?.response?.data?.message || 'Upload failed';
		console.error('[AVATAR_UPLOAD]', err?.response?.data || err.message);
		return json({ error: message }, { status });
	}
}

// DELETE /api/v1/user/avatar  - remove the uploaded avatar.
export async function DELETE({ locals }) {
	const user = await locals.session.user.get();
	if (!user) {
		return json({ error: 'Unauthorized' }, { status: 401 });
	}

	try {
		await ApiServerClient.delete('/user/avatar', {
			headers: { 'X-Api-Key': user.api_key }
		});
		await locals.session.user.update({ key: 'profile_image', value: null });
		return json({ success: true });
	} catch (err) {
		console.error('[AVATAR_DELETE]', err?.response?.data || err.message);
		return json({ error: 'Failed to remove display picture' }, { status: 500 });
	}
}
