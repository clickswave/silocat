import axios from 'axios';
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

// USER MANAGEMENT
export let ApiServerClient = axios.create({
	baseURL: INTERNAL_API_URL,
	headers: { [WEB_SERVER_HEADER]: WEB_SERVER_COMMUNICATION_SECRET }
});

export const ApiServerRoutes = {

	validateShadowUser: '/validate-shadow-user',
	report: '/report',
	earlyAccess: '/auth/early-access',

	login: '/user/login',
	registerPersonal: '/user/register-personal',
	changePassword: '/user/change-password',

	createFile: '/file/create-files',


	getProfile: '/user/profile',
	rotateKey: '/user/rotate-key',
	markChunkAsComplete: '/file/mark-chunk-complete',
	markChunkAsUploading: '/file/mark-chunk-uploading',
	downloadFile: '/file/download-file',
	fetchChunks: '/file/fetch-chunks',
	fetchFiles: '/file/fetch-files',
	// createFile: '/file/new-file',
	startChunkUpload: '/file/start-chunk-upload',
	stopChunkUpload: '/file/stop-chunk-upload',
	startChunkDownload: '/file/start-chunk-download',
	startChunkDelete: '/file/start-chunk-delete',
	deleteFile: '/file/delete-files',
	listFiles: '/file/list-files',
	createFolder: '/file/create-folders',
	listFolders: '/file/fetch-folders',
	getFolder: '/folder/list',
	deleteFolder: '/file/delete-folders',
	fetchStorageStats: '/user/storage-stats',
	fetchResource: '/file/fetch-resource',
};
