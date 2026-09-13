import { ApiServerClient, ApiServerRoutes } from '$lib/network.js';
import { clientIpHeaders } from '$lib/server/client-ip.js';

/**
 * Register an anonymous visitor's browser key before it is used as an identity.
 *
 * A shadow key is generated in the browser's localStorage and means nothing to
 * api_switch until `/validate-shadow-user` has minted the matching
 * `anonymous_users` row. Until then `resolve_identity` cannot resolve it, so any
 * route that requires a caller answers 401.
 *
 * This used to live inside the file-create proxy alone, which put the one call
 * that registers a key behind the one endpoint that is not reached first. The
 * drop page creates its batch folder BEFORE it creates any file, so on a first
 * ever visit the folder call arrived with an unregistered key, 401'd, and the
 * upload fell back to loose files with no folder. The visitor was then shown no
 * share link at all, because the link is set from the folder in the multi-file
 * path. Their bytes uploaded and they had no way to reach them. It worked on the
 * second upload from the same browser, which is why it was invisible to anyone
 * testing with a browser they had already used.
 *
 * Returns `{ success, user }` or `{ success: false, error }` shaped like the
 * api_switch envelope, so callers can hand the error straight back.
 */
export async function validateShadowRequest(event) {
	const { request } = event;
	const headers = request.headers;
	const ipHeaders = clientIpHeaders(event);

	const payload = {
		api_key: request.headers.get('X-Api-Key'),
		user_agent: headers.get('user-agent'),
		ip: ipHeaders['X-Client-IP'] || '127.0.0.1',
		// Cloudflare geo headers, best effort: absent off the edge.
		geo: {
			country: headers.get('cf-ipcountry') || 'Unknown',
			region: headers.get('cf-region') || 'Unknown',
			city: headers.get('cf-city') || 'Unknown',
			latitude: headers.get('cf-iplatitude') || 'Unknown',
			longitude: headers.get('cf-iplongitude') || 'Unknown',
			asn: headers.get('cf-asn') || 'Unknown',
			isp: headers.get('cf-isp') || 'Unknown'
		}
	};

	try {
		const validate = await ApiServerClient.post(ApiServerRoutes.validateShadowUser, payload, {
			headers: ipHeaders
		}).then((res) => res.data);
		return { success: true, user: validate?.user };
	} catch (e) {
		console.error('[SHADOW_VALIDATE]', e.message, e.code, e.response?.status);
		const error = e?.response?.data || {
			status: 500,
			message: 'Could not validate user',
			errors: 'Something went wrong trying to validate user, please try again later',
			data: {}
		};
		return { success: false, error };
	}
}
