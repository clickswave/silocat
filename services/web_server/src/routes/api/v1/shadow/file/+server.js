import { json } from '@sveltejs/kit';
import { ApiServerClient, ApiServerRoutes } from '$lib/network.js';

function getClientIp(headers) {
	// Cloudflare
	const cfIp = headers.get('cf-connecting-ip');
	if (cfIp) return cfIp;

	// Reverse proxies
	const xff = headers.get('x-forwarded-for');
	if (xff) return xff.split(',')[0].trim();

	return null;
}

async function validateRequest({ request }) {

	let headers = request.headers;

	// 1. API key enforcement
	const apiKey = request.headers.get('X-Api-Key');
	const clientIp = getClientIp(headers) || "127.0.0.1";
	const userAgent = headers.get('user-agent');
	// Cloudflare geo headers (best case)
	const geo = {
		country: headers.get('cf-ipcountry') || "Unknown",
		region: headers.get('cf-region') || "Unknown",
		city: headers.get('cf-city') || "Unknown",
		latitude: headers.get('cf-iplatitude') || "Unknown",
		longitude: headers.get('cf-iplongitude') || "Unknown",
		asn: headers.get('cf-asn') || "Unknown",
		isp: headers.get('cf-isp') || "Unknown"
	};

	console.log("[log 1]");

	let payload = {
		api_key: apiKey, user_agent: userAgent, ip: clientIp, geo
	};
	try {
		let validate = await ApiServerClient.post(ApiServerRoutes.validateShadowUser, payload, {
			headers: { 'CF-Connecting-IP': clientIp }
		}).then(res => res.data);
		let { user } = validate;
		return { success: true, user };
	} catch (e) {
		console.error("Raw Validation Error:", e.message, e.code, e.response?.status, e.response?.data);
		let error = e?.response?.data || {
			status: 500,
			message: 'Could not validate user',
			errors: 'Something went wrong trying to validate user, please try again later',
			data: {}
		};
		console.log({ error });
		return { success: false, error };
	}
}

export async function POST({ request, locals }) {

	let { success, user, error } = await validateRequest({ request, headers: locals.headers });
	if (!success) {
		return json({ error: error.message || 'Unauthorized' }, { status: error.status || 401 });
	}

	const MAX_SIZE_ANON = 20 * 1024 * 1024 * 1024; // 20GB
	const MAX_SIZE_AUTH = 50 * 1024 * 1024 * 1024; // 50GB

	// Determine limit
	// Assuming 'anonymous' account type for shadow users.
	// If validated user has account_type 'anonymous' (or similar), apply lower limit.
	// If user is missing (shouldn't happen if success=true but strictly speaking), apply lower.
	const isAnonymous = !user || user.account_type === 'anonymous';
	const limit = isAnonymous ? MAX_SIZE_ANON : MAX_SIZE_AUTH;

	let body;
	try {
		body = await request.json();
		// Inject owner API key from headers (for anonymous management)
		body.owner_api_key = await validateRequest({ request, headers: locals.headers }).then(r => r.user?.api_key || request.headers.get('X-Api-Key'));

		if (body.file_size && body.file_size > limit) {
			const limitGb = limit / (1024 * 1024 * 1024);
			return json({
				error: `File size exceeds the ${limitGb}GB limit for ${isAnonymous ? 'anonymous' : 'authenticated'} users.`
			}, { status: 413 });
		}

	} catch (e) {
		console.log(e);
		return json({ error: 'Invalid JSON body' }, { status: 400 });
	}

	try {
		const clientIp = getClientIp(request.headers) || '127.0.0.1';
		const sessionUser = await locals.session.user.get();
		const apiKey = sessionUser?.api_key || request.headers.get('X-Api-Key') || undefined;
		let response = await ApiServerClient.post(ApiServerRoutes.createFile, body, {
			headers: { 'CF-Connecting-IP': clientIp, 'X-Api-Key': apiKey }
		}).then(res => res.data);
		return json(response);
	} catch (err) {
		console.error('[CREATE_FILE_METADATA]', err?.response?.status, err?.response?.data || err.message);
		// Pass through upstream errors (e.g. 403 "You are banned") so the UI can react.
		const up = err?.response?.data;
		return json({ error: up?.message || 'Failed to create file metadata', banned: up?.data?.banned }, { status: err?.response?.status || 500 });
	}
}
