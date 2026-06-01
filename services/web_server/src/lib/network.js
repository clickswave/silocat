import axios from 'axios';
import { env } from '$env/dynamic/private';
const { AUTHORITY_SIGN, INTERNAL_API_URL } = env;
console.log({AUTHORITY_SIGN, INTERNAL_API_URL});

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
	headers: { 'X-Authority-Sign': AUTHORITY_SIGN }
});

export const ApiServerRoutes = {

	validateShadowUser: '/validate-shadow-user',
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
