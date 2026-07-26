import axios from 'axios';
import { env } from '$env/dynamic/private';

const { INFRA_COMMUNICATION_SECRET, INTERNAL_API_URL, INFRA_COMMUNICATION_SECRET_HEADER } = env;

// Header name for the backend-to-backend secret. Configurable so the published
// source does not fingerprint a deployment's wire protocol; the name is not a
// secret in itself, the value is. Must match api_switch's
// `INFRA_COMMUNICATION_SECRET_HEADER`.
//
// This module is `$env/dynamic/private`, so none of this ever reaches a browser:
// the sign is attached during SSR and the client never sees it.
const INFRA_HEADER = INFRA_COMMUNICATION_SECRET_HEADER?.trim() || 'X-Authority-Sign';

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
	headers: { [INFRA_HEADER]: INFRA_COMMUNICATION_SECRET }
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
