import axios from 'axios';
import { json } from '@sveltejs/kit';
import { env } from '$env/dynamic/private';

const { WEB_SERVER_COMMUNICATION_SECRET, INTERNAL_API_URL, WEB_SERVER_COMMUNICATION_HEADER } = env;

// This backend's caller identity: its own header + secret, matched by api_switch
// against the WEB_SERVER caller. The header name is configurable (default
// X-Web-Server) and must equal api_switch's `WEB_SERVER_COMMUNICATION_HEADER`;
// the name is not a secret, the value is.
//
// This module is `$env/dynamic/private`, so none of this ever reaches a browser:
// the secret is attached during SSR and the client never sees it.
const WEB_SERVER_HEADER = WEB_SERVER_COMMUNICATION_HEADER?.trim() || 'X-Web-Server';

// USER MANAGEMENT SERVER DOWN ERROR
export const ApiServerError = {
	status: 500,
	message: 'Something went wrong',
	errors: ['Last request was not successful. If the problem persists, please contact support.'],
	data: {}
};

/**
 * Turn a failed `ApiServerClient` call into what the browser should actually be
 * told, and use it in every proxy's catch.
 *
 * api_switch maps its envelope status onto the HTTP code, so ANY non-2xx makes
 * axios throw and land in the catch. Each proxy used to answer a flat 500 there,
 * which discarded the one thing the caller needed: a 404 for a thing that is
 * gone, a 401 for a password or session that was refused, a 410 for a link that
 * is spent. Pages branch on exactly those, so those branches were dead code, and
 * any status check written *after* the await in a proxy was dead too.
 *
 * A genuine transport failure (api_switch down, DNS, timeout) has no
 * `err.response` and is the only case that is really a 500 here, so that is the
 * only case that still reports one.
 *
 * @param {unknown} err     the thrown axios error
 * @param {string}  tag     log prefix, e.g. '[FETCH_CHUNKS]'
 * @param {string}  message what to say when there is no upstream response
 */
export function proxyError(err, tag, message) {
	const upstream = err?.response?.data;
	if (upstream) return json(upstream, { status: err.response.status });
	console.error(tag, err);
	return json({ error: message }, { status: 500 });
}

// USER MANAGEMENT
export let ApiServerClient = axios.create({
	baseURL: INTERNAL_API_URL,
	headers: { [WEB_SERVER_HEADER]: WEB_SERVER_COMMUNICATION_SECRET }
});

// Only the endpoints something actually calls. The map had accumulated eleven
// entries with no call sites, several pointing at handlers that no longer exist,
// which makes it useless as a description of the surface.
export const ApiServerRoutes = {

	validateShadowUser: '/validate-shadow-user',
	report: '/report',

	login: '/user/login',
	registerPersonal: '/user/register-personal',

	createFile: '/file/create-files',

	markChunkAsComplete: '/file/mark-chunk-complete',
	fetchChunks: '/file/fetch-chunks',
	fetchFiles: '/file/fetch-files',
	deleteFile: '/file/delete-files',
	listFiles: '/file/list-files',
	createFolder: '/file/create-folders',
	listFolders: '/file/fetch-folders',
	deleteFolder: '/file/delete-folders',
	fetchStorageStats: '/user/storage-stats',
	fetchResource: '/file/fetch-resource',
};
