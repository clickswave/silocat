<script>
	import FolderCard from '$lib/components/FolderCard.svelte';
	import StatsCard from '$lib/components/StatsCard.svelte';
	import FileCard from '$lib/components/FileCard.svelte';
	import { flip } from 'svelte/animate'; // Add flip import for smooth list reordering animation
	import { FrontendClient } from '$lib/frontendClient.js';
	import { browser } from '$app/environment';
	import { createQuery } from '@tanstack/svelte-query';

	let { data } = $props();

	async function fetchFilesFn() {
		try {
			let { data } = await FrontendClient.get('/api/v1/sanctum/file/list', {
				params: { folder_id: currentFolderId || undefined }
			});

			if (data?.status === 200) {
				return data?.data?.files;
			} else {
				throw (
					data || {
						status: 500,
						message: 'Unknown error fetching files',
						errors: ['An unknown error occurred while fetching files.'],
						data: {}
					}
				);
			}
		} catch (e) {
			console.error('[--] Error fetching files:', e);
			throw {
				message: 'Error fetching files'
			};
		}
	}

	const fetchFiles = createQuery(() => ({
		queryKey: ['fetchFiles'],
		queryFn: fetchFilesFn,
		enabled: browser
	}));

	async function fetchFoldersFn() {
		try {
			console.log('[FetchFolders] Requesting root folders...');
			console.log('[FetchFolders] Requesting folders for parent:', currentFolderId);
			let { data } = await FrontendClient.post('/api/v1/sanctum/folder/list', {
				parent_id: currentFolderId || null
			}); // Filter by parent
			console.log('[FetchFolders] Response:', data);

			// Backend returns: { status: 200, data: { folders: [...] } }
			if (data && data.data && data.data.folders) {
				return data.data.folders;
			}
			return [];
		} catch (e) {
			console.error('Error fetching folders:', e);
			return [];
		}
	}

	const fetchFolders = createQuery(() => ({
		queryKey: ['fetchFolders'],
		queryFn: fetchFoldersFn,
		enabled: browser
	}));

	async function fetchStorageStatsFn() {
		try {
			let { data } = await FrontendClient.get('/api/v1/sanctum/user/storage');
			if (data?.success) {
				return data.success;
			}
			return { total: 0, used: 0, free: 0 };
		} catch (e) {
			console.error('Error fetching storage stats:', e);
			return { total: 0, used: 0, free: 0 };
		}
	}

	const fetchStorageStats = createQuery(() => ({
		queryKey: ['fetchStorageStats'],
		queryFn: fetchStorageStatsFn,
		enabled: browser
	}));

	// Process API data for display
	let folders = $derived(
		fetchFolders?.data?.map((f) => ({
			id: f.id,
			name: f.name,
			count: f.count || 0,
			color: 'blue', // Default color
			starred: f.starred
		})) || []
	);

	// Star Logic
	async function handleStar(item, type) {
		const newStatus = !item.starred;
		try {
			await axios.post(`/api/v1/sanctum/${type}/star`, {
				[type === 'file' ? 'file_id' : 'folder_id']: item.id,
				starred: newStatus
			});
			toast.success(newStatus ? 'Added to starred' : 'Removed from starred');
			queryClient.invalidateQueries({ queryKey: ['fetchFiles'] });
			queryClient.invalidateQueries({ queryKey: ['fetchFolders'] });
			queryClient.invalidateQueries({ queryKey: ['fetchStarredFiles'] });
			queryClient.invalidateQueries({ queryKey: ['fetchStarredFolders'] });
		} catch (e) {
			console.error('Star failed:', e);
			toast.error('Failed to update star status');
		}
	}

	function handleShare(item, type) {
		itemToShare = type ? { ...item, type } : item;
		showShareModal = true;
	}

	import Icon from '@iconify/svelte';
	import { createMutation, useQueryClient } from '@tanstack/svelte-query';
	import { toast } from 'svelte-sonner';
	import axios from 'axios';
	import {
		encryptChunk,
		deriveKeyFromPassword,
		generateSalt,
		generateNonce,
		decryptChunk
	} from '$lib/chacha.js';
	import sodium from 'libsodium-wrappers-sumo';
	import { fade, scale } from 'svelte/transition';
	import JSZip from 'jszip';

	import InputModal from '$lib/components/InputModal.svelte';
	import ConfirmModal from '$lib/components/ConfirmModal.svelte';
	import ShareModal from '$lib/components/ShareModal.svelte';

	// ... existing imports ...

	// Helper to format bytes
	// ...

	// State for Modals
	let fileInput = $state(null);
	let folderInput = $state(null);
	let showCreateFolderModal = $state(false);
	let showRenameFolderModal = $state(false);
	let showDeleteFolderModal = $state(false);

	let folderToRename = $state(null);
	let folderToDelete = $state(null);
	let newFolderName = $state('');

	let showDecryptModal = $state(false);
	let fileToDecrypt = $state(null);
	let decryptionPassword = $state('');

	let showShareModal = $state(false);
	let itemToShare = $state(null);

	// --- Upload Logic ---
	// ...

	// --- Folder Navigation State ---
	let currentFolderId = $state(null);
	let folderPath = $state([{ id: null, name: 'Home' }]);

	function handleFolderClick(folder) {
		currentFolderId = folder.id;
		folderPath = [...folderPath, { id: folder.id, name: folder.name }];
		refreshView();
	}

	function navigateToBreadcrumb(index) {
		// If clicking current, do nothing
		if (index === folderPath.length - 1) return;

		const target = folderPath[index];
		currentFolderId = target.id;
		folderPath = folderPath.slice(0, index + 1);
		refreshView();
	}

	function navigateUp() {
		if (folderPath.length <= 1) return;
		navigateToBreadcrumb(folderPath.length - 2);
	}

	function refreshView() {
		queryClient.invalidateQueries({ queryKey: ['fetchFiles'] });
		queryClient.invalidateQueries({ queryKey: ['fetchFolders'] });
	}

	// --- Folder Logic ---
	async function startDownloadFolderZip(folder) {
		const toastId = toast.loading('Preparing zip download...');

		try {
			// 1. Fetch files for the folder
			let { data } = await FrontendClient.get('/api/v1/sanctum/file/list', {
				params: { folder_id: folder.id }
			});

			if (!data?.success?.data?.files) {
				throw new Error('Failed to fetch folder contents');
			}

			const folderFiles = data.success.data.files;

			if (folderFiles.length === 0) {
				toast.error('Folder is empty');
				toast.dismiss(toastId);
				return;
			}

			// 2. Prepare Zip
			await sodium.ready;
			const zip = new JSZip();
			let processedCount = 0;

			// Check password requirement logic
			const hasEncrypted = folderFiles.some((f) => f.encrypted);
			if (hasEncrypted && !password) {
				// If we don't have a global password state populated (e.g. from a recent unlock),
				// we might fail. For now, we reuse the `password` variable which is utilized in the upload modal
				// OR `decryptionPassword` from download modal?
				// The file card usage uses `decryptionPassword` and `showDecryptModal`.
				// Let's rely on user unlocking a file first or just prompt if we want to be fancy.
				// For this MVP step, we'll error if encrypted and no password available in `decryptionPassword`.
				if (!decryptionPassword) {
					toast.error(
						'Please decrypt a file first to unlock session keys, or Folder is encrypted.'
					);
					toast.dismiss(toastId);
					return;
				}
			}

			for (const file of folderFiles) {
				try {
					// Fetch chunks
					const chunksRes = await axios.post('/api/v1/shadow/file/fetch-chunks', {
						file_id: file.id
					});
					const chunks = chunksRes.data.data.chunks;

					if (!chunks || chunks.length === 0) continue;

					let fileKey = null;
					if (file.encrypted) {
						const firstChunk = chunks[0];
						if (!firstChunk.salt) throw new Error(`File ${file.name} missing salt.`);
						const saltBytes = Uint8Array.from(atob(firstChunk.salt), (c) => c.charCodeAt(0));
						// Using `decryptionPassword` if set
						fileKey = await deriveKeyFromPassword(decryptionPassword, saltBytes);
					}

					const downloadedChunks = [];
					for (const chunk of chunks) {
						const chunkData = await axios.get(chunk.presigned_url, { responseType: 'arraybuffer' });
						let dataBytes = new Uint8Array(chunkData.data);

						if (file.encrypted) {
							if (!chunk.nonce) throw new Error(`Chunk missing nonce for ${file.name}`);
							const nonceBytes = Uint8Array.from(atob(chunk.nonce), (c) => c.charCodeAt(0));
							dataBytes = await decryptChunk(dataBytes, fileKey, nonceBytes);
						}
						downloadedChunks.push(dataBytes);
					}

					const fileBlob = new Blob(downloadedChunks);
					zip.file(file.name, fileBlob);
					processedCount++;
				} catch (e) {
					console.error(`Failed to process file ${file.name}`, e);
				}
			}

			if (processedCount === 0) {
				throw new Error('No files could be processed successfully');
			}

			const zipContent = await zip.generateAsync({ type: 'blob' });
			const zipName = `${folder.name}.zip`;
			const url = URL.createObjectURL(zipContent);

			const a = document.createElement('a');
			a.href = url;
			a.download = zipName;
			document.body.appendChild(a);
			a.click();
			document.body.removeChild(a);
			URL.revokeObjectURL(url);

			toast.success('Download complete');
		} catch (e) {
			console.error('Zip download failed:', e);
			toast.error('Failed to download zip: ' + e.message);
		} finally {
			toast.dismiss(toastId);
		}
	}

	async function handleCreateFolder() {
		showCreateFolderModal = true;
	}

	async function confirmCreateFolder(name) {
		showCreateFolderModal = false;
		if (!name) return;

		try {
			console.log('Creating folder:', name);
			const res = await axios.post('/api/v1/sanctum/folder/create', {
				name,
				parent_id: currentFolderId
			});
			console.log('Folder created response:', res);

			if (res.data && res.data.data && res.data.data.id) {
				toast.success('Folder created');
				queryClient.invalidateQueries({ queryKey: ['fetchFolders'] });
			} else {
				console.error('Invalid response structure:', res.data);
				throw new Error('Invalid response from server');
			}
		} catch (e) {
			console.error('Folder creation failed:', e);
			toast.error('Failed to create folder: ' + (e.response?.data?.message || e.message));
		}
	}

	function handleRenameFolder(folder) {
		folderToRename = folder;
		newFolderName = folder.name; // Initial value
		showRenameFolderModal = true;
	}

	async function confirmRenameFolder(newName) {
		showRenameFolderModal = false;
		if (!newName || newName === folderToRename.name) return;

		try {
			await axios.post('/api/v1/sanctum/folder/update', {
				folder_id: folderToRename.id,
				new_name: newName
			});
			toast.success('Folder renamed');
			queryClient.invalidateQueries({ queryKey: ['fetchFolders'] });
		} catch (e) {
			console.error('Rename failed:', e);
			toast.error('Failed to rename folder');
		}
	}

	// Delete Folder Logic
	function handleDeleteFolder(folder) {
		folderToDelete = folder;
		// Fetch stats before showing modal? Or show modal with loading?
		// Let's fetch stats first to show "X items will be deleted" immediately
		// Reset stats
		deletedItemCount = null;

		// Ideally we show a loading toast or just wait a bit?
		// Or we open modal and it shows "Calculating items..."
		// Let's open modal and trigger fetch.
		showDeleteFolderModal = true;
		fetchDeletionStats(folder.id);
	}

	let deletedItemCount = $state(null);

	async function fetchDeletionStats(folderId) {
		try {
			const res = await axios.post('/api/v1/sanctum/folder/stats', { folder_id: folderId });
			if (res.data?.data) {
				deletedItemCount = res.data.data.total_items;
			}
		} catch (e) {
			console.error('Failed to fetch folder stats', e);
			deletedItemCount = 'unknown';
		}
	}

	async function confirmDeleteFolder() {
		showDeleteFolderModal = false;
		if (!folderToDelete) return;

		try {
			await axios.post('/api/v1/sanctum/folder/delete', { folder_id: folderToDelete.id });
			toast.success('Folder deleted');
			queryClient.invalidateQueries({ queryKey: ['fetchFolders'] });
		} catch (e) {
			console.error('Delete failed:', e);
			toast.error('Failed to delete folder');
		}
	}

	// Helper to format bytes
	function formatSize(bytes) {
		if (bytes === 0) return '0 B';
		const k = 1024;
		const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
		const i = Math.floor(Math.log(bytes) / Math.log(k));
		return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
	}

	// Helper for relative time
	function formatTime(dateString) {
		const date = new Date(dateString);
		const now = new Date();
		const diffInSeconds = Math.floor((now - date) / 1000);

		if (diffInSeconds < 60) return 'Just now';
		if (diffInSeconds < 3600) return `${Math.floor(diffInSeconds / 60)} min ago`;
		if (diffInSeconds < 86400) return `${Math.floor(diffInSeconds / 3600)} hour ago`;
		return `${Math.floor(diffInSeconds / 86400)} days ago`;
	}

	// Helper to determine icon type from mime
	function getFileType(mime) {
		if (mime.includes('image')) return 'image';
		if (mime.includes('video')) return 'video';
		if (mime.includes('audio')) return 'audio';
		if (mime.includes('pdf') || mime.includes('document')) return 'doc';
		return 'file';
	}

	const queryClient = useQueryClient();

	let files = $state([]);
	let isDragging = $state(false);
	let isUploading = $state(false);
	let showUploadModal = $state(false);
	let encryptionEnabled = $state(false);
	let password = $state('');

	// --- Multi-Select Logic ---
	let selectedItems = $state(new Set()); // Stores IDs string like "folder:123" or "file:456"

	function handleItemClick(e, item, type) {
		// e.preventDefault(); // Maybe not needed
		const key = `${type}:${item.id}`;

		if (e.ctrlKey || e.metaKey) {
			// Toggle
			const newSet = new Set(selectedItems);
			if (newSet.has(key)) {
				newSet.delete(key);
			} else {
				newSet.add(key);
			}
			selectedItems = newSet;
		} else {
			// Single select
			selectedItems = new Set([key]);
		}
	}

	// --- Drag and Drop (Move) Logic ---
	let dragTargetId = $state(null);
	let isInternalDrag = $state(false);

	function handleItemDragStart(e, item, type) {
		const key = `${type}:${item.id}`;

		// If dragging an unselected item (without ctrl), select ONLY it
		if (!selectedItems.has(key) && !e.ctrlKey && !e.metaKey) {
			selectedItems = new Set([key]);
		}

		e.dataTransfer.effectAllowed = 'move';

		// Remove ghost image
		const img = new Image();
		img.src = 'data:image/gif;base64,R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7';
		e.dataTransfer.setDragImage(img, 0, 0);

		// Prepare payload: all selected items
		const payload = [];
		for (const k of selectedItems) {
			const [t, id] = k.split(':');
			payload.push({ id, type: t });
		}

		e.dataTransfer.setData('application/silo-items', JSON.stringify(payload));
		isDragging = true;
		isInternalDrag = true;
	}

	function handleItemDragEnd(e) {
		isDragging = false;
		isInternalDrag = false;
		dragTargetId = null;
	}

	function handleItemDragOver(e, folder) {
		// Only allow dropping if dragging an item (not external file upload, though that could optionally be supported to upload INTO folder)
		// For now focus on moving items.
		e.preventDefault(); // Necessary to allow dropping
		// If we are strictly checking isInternalDrag, fine.
		if (!isInternalDrag) return;

		e.dataTransfer.dropEffect = 'move';
		dragTargetId = folder.id;
	}

	function handleBreadcrumbDragOver(e, breadcrumb) {
		e.preventDefault();
		if (!isInternalDrag) return;
		e.dataTransfer.dropEffect = 'move';
		dragTargetId = breadcrumb.id === null ? 'root' : breadcrumb.id;
	}

	async function handleBreadcrumbDrop(e, breadcrumb) {
		e.preventDefault();
		e.stopPropagation();
		dragTargetId = null;
		isDragging = false;
		isInternalDrag = false;

		// No-op check: if moving to current view's folder
		if (breadcrumb.id === currentFolderId) return;

		const data = e.dataTransfer.getData('application/silo-items');
		if (!data) return;

		try {
			const items = JSON.parse(data);
			const promises = items.map((item) => moveItem(item, breadcrumb));
			await Promise.all(promises);
		} catch (err) {
			console.error('Breadcrumb drop error:', err);
		}
	}

	function handleItemDragLeave(e) {
		// debounce or check relatedTarget to avoid flickering
		// dragTargetId = null;
	}

	async function handleItemDrop(e, targetFolder) {
		e.preventDefault();
		e.stopPropagation();
		dragTargetId = null;
		isDragging = false;
		isInternalDrag = false;

		const data = e.dataTransfer.getData('application/silo-items');
		if (!data) return;

		try {
			const items = JSON.parse(data);
			// Parallel moves
			const promises = items
				.filter((item) => {
					if (item.type === 'folder' && item.id === targetFolder.id) {
						toast.error(`Cannot move folder "${item.name || 'item'}" into itself`);
						return false;
					}
					return true;
				})
				.map((item) => moveItem(item, targetFolder));
			await Promise.all(promises);
		} catch (err) {
			console.error('Drop error:', err);
		}
	}

	async function moveItem(item, targetFolder) {
		try {
			const endpoint =
				item.type === 'file' ? '/api/v1/sanctum/file/update' : '/api/v1/sanctum/folder/update';
			let payload = {};

			if (item.type === 'file') {
				payload = { file_id: item.id, new_folder_id: targetFolder.id };
			} else {
				if (targetFolder.id === null) {
					payload = { folder_id: item.id, move_to_root: true };
				} else {
					payload = { folder_id: item.id, new_parent_id: targetFolder.id };
				}
			}

			await FrontendClient.post(endpoint, payload);
			toast.success(`Moved to ${targetFolder.name}`);
			refreshView();
		} catch (e) {
			console.error('Move failed:', e);
			toast.error('Failed to move item');
		}
	}

	// Upload Progress State
	let uploadStats = $state({
		totalBytes: 0,
		uploadedBytes: 0,
		speed: 0,
		eta: 0,
		totalProgress: 0,
		fileProgress: 0,
		chunkProgress: 0,
		startTime: 0,
		currentFileName: ''
	});

	function handleDragOver(e) {
		e.preventDefault();
		isDragging = true;
	}
	function handleDragLeave() {
		isDragging = false;
	}

	// Recursive directory scanner
	async function scanFiles(item, path = '') {
		if (item.isFile) {
			return new Promise((resolve) => {
				item.file((file) => {
					// Normalize path: path (folder structure) + file name is not usually needed for upload "path" property
					// if we treat it as folder structure.
					// But for our logic, we need the folder path separate from file.
					resolve([{ file, path }]);
				});
			});
		} else if (item.isDirectory) {
			const dirReader = item.createReader();
			const entries = await new Promise((resolve) => {
				dirReader.readEntries(resolve);
			});
			let files = [];
			for (const entry of entries) {
				files = [...files, ...(await scanFiles(entry, path ? `${path}/${item.name}` : item.name))];
			}
			return files;
		}
		return [];
	}

	async function handleDrop(e) {
		e.preventDefault();
		isDragging = false;

		// Enhanced folder scanning support
		if (e.dataTransfer.items) {
			let newFiles = [];
			const items = Array.from(e.dataTransfer.items);
			for (const item of items) {
				if (item.kind === 'file') {
					const entry = item.webkitGetAsEntry ? item.webkitGetAsEntry() : null;
					if (entry) {
						newFiles = [...newFiles, ...(await scanFiles(entry))];
					} else {
						// Fallback for non-webkit or basic files
						const file = item.getAsFile();
						if (file) newFiles.push({ file, path: '' });
					}
				}
			}
			files = [...files, ...newFiles];
		} else if (e.dataTransfer.files) {
			// Fallback standard file drop
			files = [...files, ...Array.from(e.dataTransfer.files).map((f) => ({ file: f, path: '' }))];
		}
	}

	function handleFileSelect(e) {
		if (!e.target.files) return;

		const selected = Array.from(e.target.files);

		// Note: Standard clicking on input[type=file] usually only selects files.
		// If browser supports selecting folders here, we accept them.
		// Drag and drop is handled by handleDrop which supports both.

		const newFiles = selected.map((f) => ({
			file: f,
			path: f.webkitRelativePath ? f.webkitRelativePath.split('/').slice(0, -1).join('/') : ''
		}));

		files = [...files, ...newFiles];
		e.target.value = '';
	}

	function removeFile(index) {
		files = files.filter((_, i) => i !== index);
	}

	function generatePassword() {
		const chars = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%^&*';
		let pass = '';
		for (let i = 0; i < 16; i++) pass += chars.charAt(Math.floor(Math.random() * chars.length));
		return pass;
	}

	// --- Upload Logic ---
	const CHUNK_SIZE = 100 * 1024 * 1024; // 100MB

	// Worker Integration
	import CryptoWorker from '$lib/workers/crypto.worker.js?worker';

	let cryptoWorker;
	let workerCallbacks = new Map();

	function initWorker() {
		if (!cryptoWorker) {
			cryptoWorker = new CryptoWorker();
			cryptoWorker.onmessage = (e) => {
				const { id, status, result, error } = e.data;
				if (id && workerCallbacks.has(id)) {
					const { resolve, reject } = workerCallbacks.get(id);
					if (status === 'success') resolve(result);
					else reject(new Error(error));
					workerCallbacks.delete(id);
				}
			};
		}
	}

	function callWorker(type, payload, transferables = []) {
		initWorker();
		return new Promise((resolve, reject) => {
			const id = Math.random().toString(36).substring(7);
			workerCallbacks.set(id, { resolve, reject });
			cryptoWorker.postMessage({ id, type, payload }, transferables);
		});
	}

	async function getFileChecksum(file) {
		return callWorker('hashFile', { file, chunkSize: CHUNK_SIZE });
	}

	async function deriveKeyFromPasswordWorker(password, salt) {
		// salt is Uint8Array, we pass it directly.
		return callWorker('deriveKey', { password, salt });
	}

	async function encryptChunkWorker(chunkBuffer, key, nonce) {
		// Pass chunkBuffer as transferable
		return callWorker('encryptChunk', { chunk: chunkBuffer, key, nonce }, [chunkBuffer.buffer]);
	}

	const uploadMutation = createMutation(() => ({
		mutationFn: async ({ file, folderId, onProgress }) => {
			await sodium.ready; // Still need main thread sodium for random generation (nonce/salt)?
			// Actually, random generation is fast and non-blocking.
			// crypto_hash and crypto_secretbox are the heavy ones.

			const fileChecksum = await getFileChecksum(file);
			let key = null;
			let salt = null;

			if (encryptionEnabled) {
				if (!password) throw new Error('Password required for encrypted upload');
				salt = generateSalt(); // Fast
				key = await deriveKeyFromPasswordWorker(password, salt);
			}

			const totalChunks = Math.ceil(file.size / CHUNK_SIZE);
			const chunksMeta = [];

			for (let i = 0; i < totalChunks; i++) {
				const start = i * CHUNK_SIZE;
				const end = Math.min(start + CHUNK_SIZE, file.size);
				let chunkNonce = null;
				let chunkSalt = null;
				if (encryptionEnabled) {
					chunkNonce = generateNonce(); // Fast
					chunkSalt = salt;
				}
				chunksMeta.push({
					start,
					end,
					size: end - start,
					checksum: 'pending',
					nonce: chunkNonce ? btoa(String.fromCharCode(...chunkNonce)) : null,
					salt: chunkSalt ? btoa(String.fromCharCode(...chunkSalt)) : null,
					_rawNonce: chunkNonce
				});
			}

			const payload = {
				storage_type: 'sanctum', // Logged in user = sanctum
				file_encrypted: encryptionEnabled,
				file_name: file.name,
				file_mime: file.type || 'application/octet-stream',
				file_size: file.size,
				chunks: chunksMeta.map((c) => ({
					start: c.start,
					end: c.end,
					size: c.size,
					checksum: c.checksum,
					salt: c.salt,
					nonce: c.nonce
				})),
				sha256_checksum: fileChecksum,
				blake3_checksum: '',
				public_access: !encryptionEnabled,
				folder_id: folderId
			};

			const res = await axios.post('/api/v1/sanctum/file', payload);
			if (res.data.status !== 200 && res.data.status !== 201)
				throw new Error(res.data.message || 'Upload failed');

			const serverChunks = res.data.data.chunks;
			let fileUploadedBytes = 0;

			for (let i = 0; i < totalChunks; i++) {
				const chunkMeta = chunksMeta[i];
				const serverChunk = serverChunks[i];
				const chunkBlob = file.slice(chunkMeta.start, chunkMeta.end);
				const chunkBuffer = new Uint8Array(await chunkBlob.arrayBuffer());

				let dataToUpload = chunkBuffer;
				if (encryptionEnabled) {
					// Offload encryption to worker
					dataToUpload = await encryptChunkWorker(chunkBuffer, key, chunkMeta._rawNonce);
				}

				console.log(`[Upload] Chunk ${i} URL:`, serverChunk.presigned_url);
				console.log(`[Upload] Chunk ${i} Size:`, dataToUpload.byteLength);

				const MAX_RETRIES = 3;
				let lastError;

				for (let attempt = 1; attempt <= MAX_RETRIES; attempt++) {
					try {
						await axios.put(serverChunk.presigned_url, dataToUpload, {
							headers: { 'Content-Type': 'application/octet-stream' },
							onUploadProgress: (progressEvent) => {
								if (onProgress)
									onProgress(
										(progressEvent.loaded / progressEvent.total) * 100,
										fileUploadedBytes + progressEvent.loaded
									);
							}
						});
						lastError = null;
						break; // Success
					} catch (e) {
						lastError = e;
						console.error(`[Upload] Chunk ${i} Attempt ${attempt} failed.`, {
							url: serverChunk.presigned_url,
							status: e.response?.status,
							code: e.code,
							message: e.message,
							response: e.response?.data
						});
						if (attempt < MAX_RETRIES) await new Promise((r) => setTimeout(r, 1000 * attempt));
					}
				}

				if (lastError) {
					throw new Error(
						`Chunk ${i} upload failed after ${MAX_RETRIES} attempts: ${lastError.message}`
					);
				}
				fileUploadedBytes += chunkMeta.size;
				await axios.post('/api/v1/sanctum/file/mark-chunk-complete', { chunk_id: serverChunk.id });
			}
			return { ...res.data };
		},
		onError: (error) => {
			console.error('Upload error:', error);
			toast.error(`Failed to upload: ${error.message}`);
		}
		// Removed onSuccess from here to prevent premature closing in loop
	}));

	// Folder Cache to avoid spamming create for same path
	let folderCache = new Map();

	async function ensureFolderExists(path) {
		if (!path || path === '.' || path === '') return null;
		if (folderCache.has(path)) return folderCache.get(path);

		const parts = path.split('/').filter((p) => p);
		if (parts.length === 0) return null;

		let currentParentId = null;
		let currentPath = '';

		for (const part of parts) {
			currentPath = currentPath ? `${currentPath}/${part}` : part;

			// Optimization: Check cache for partial path
			if (folderCache.has(currentPath)) {
				currentParentId = folderCache.get(currentPath);
				continue;
			}

			try {
				// Try create, backend should handle "already exists" or we handle error
				const res = await axios.post('/api/v1/sanctum/folder/create', {
					name: part,
					parent_id: currentParentId || currentFolderId // Use current folder if creating in view
				});

				// Backend returns: { data: { folder: { id: ..., name: ... } } }
				if (res.data?.data?.folder?.id) {
					currentParentId = res.data.data.folder.id;
					folderCache.set(currentPath, currentParentId);
				} else if (res.data?.data?.id) {
					// Fallback in case I'm wrong and it is flat
					currentParentId = res.data.data.id;
					folderCache.set(currentPath, currentParentId);
				}
			} catch (e) {
				// If failed, check if it's "already exists" kind of error
				// In a real app we'd fetch or use a "find_by_name" endpoint.
				// Assuming creation might fail if exists? Or maybe backend allows duplicates?
				// For now, let's try to fetch if create fails or assumes success.
				console.warn(`Could not create folder ${part}, trying to find existing...`, e);

				// Fallback: Fetch folders in parent and find one with matching name
				try {
					const listRes = await axios.post('/api/v1/sanctum/folder/list', {
						parent_id: currentParentId
					});
					const existing = listRes.data?.data?.folders?.find((f) => f.name === part);
					if (existing) {
						currentParentId = existing.id;
						folderCache.set(currentPath, currentParentId);
					} else {
						throw new Error(`Folder ${part} creation failed and not found.`);
					}
				} catch (findErr) {
					console.error('Critical folder error:', findErr);
					throw findErr;
				}
			}
		}
		return currentParentId;
	}

	async function startUpload() {
		if (files.length === 0) return;
		isUploading = true;
		folderCache.clear(); // Reset cache for new upload session

		uploadStats.startTime = Date.now();
		// Calculate total size using the file object inside our wrapper if needed
		uploadStats.totalBytes = files.reduce((acc, f) => acc + (f.file ? f.file.size : f.size), 0);

		let globalUploadedBytesBase = 0;
		let successCount = 0;
		let failCount = 0;

		try {
			for (const fileItem of files) {
				try {
					// Determine file and target folder
					const file = fileItem.file || fileItem; // Handle {file, path} wrapper or raw File
					const relativePath =
						fileItem.path ||
						(file.webkitRelativePath
							? file.webkitRelativePath.split('/').slice(0, -1).join('/')
							: null);

					uploadStats.currentFileName = file.name;

					let folderId = currentFolderId;
					if (relativePath) {
						try {
							folderId = await ensureFolderExists(relativePath);
						} catch (err) {
							console.error('Folder creation failed for', relativePath, err);
							// Continue to upload to root? OR skip?
							// Let's skip to keep integrity
							throw new Error(`Failed to create folder structure: ${err.message}`);
						}
					}

					await uploadMutation.mutateAsync({
						file,
						folderId,
						onProgress: (chunkPct, fileBytes) => {
							uploadStats.chunkProgress = chunkPct;
							uploadStats.fileProgress = (fileBytes / file.size) * 100;
							const currentTotal = globalUploadedBytesBase + fileBytes;
							uploadStats.totalProgress = (currentTotal / uploadStats.totalBytes) * 100;
						}
					});
					successCount++;
					globalUploadedBytesBase += file.size;
				} catch (e) {
					failCount++;
					console.error(`Failed to upload ${fileItem.name || 'file'}:`, e);
					toast.error(`Failed to upload ${fileItem.name || 'file'}: ${e.message}`);
				}
			}

			if (successCount > 0) {
				toast.success(`Upload complete! (${successCount} files)`);
				queryClient.invalidateQueries({ queryKey: ['fetchFiles'] });
				queryClient.invalidateQueries({ queryKey: ['fetchStorageStats'] });
				queryClient.invalidateQueries({ queryKey: ['fetchFolders'] }); // Also refresh folders!
				files = [];
				showUploadModal = false;
			} else if (failCount > 0) {
				// Keep modal open if all failed?
				toast.error('All uploads failed.');
			}
		} finally {
			isUploading = false;
		}
	}

	// --- Download Logic ---
	// --- Download Logic ---
	async function handleDownload(file) {
		if (file.encrypted) {
			fileToDecrypt = file;
			decryptionPassword = '';
			showDecryptModal = true;
			return;
		}
		processDownload(file);
	}

	async function confirmDecrypt(password) {
		showDecryptModal = false;
		if (!fileToDecrypt) return;
		processDownload(fileToDecrypt, password);
	}

	async function processDownload(file, password = null) {
		toast.info(`Starting download for ${file.name}...`);
		try {
			await sodium.ready;
			let fileKey = null;

			const chunksRes = await axios.post('/api/v1/sanctum/file/fetch-chunks', { file_id: file.id });
			const chunks = chunksRes.data.data.chunks;
			if (!chunks || chunks.length === 0) throw new Error('No chunks found');

			if (file.encrypted) {
				if (!password) throw new Error('Password required for encrypted file');
				const firstChunk = chunks[0];
				if (!firstChunk.salt) throw new Error('Encrypted file missing salt');
				const saltBytes = Uint8Array.from(atob(firstChunk.salt), (c) => c.charCodeAt(0));
				fileKey = await deriveKeyFromPassword(password, saltBytes);
			}

			const downloadedChunks = [];
			for (let i = 0; i < chunks.length; i++) {
				const chunk = chunks[i];
				const chunkData = await axios.get(chunk.presigned_url, { responseType: 'arraybuffer' });
				let dataBytes = new Uint8Array(chunkData.data);
				if (file.encrypted) {
					const nonceBytes = Uint8Array.from(atob(chunk.nonce), (c) => c.charCodeAt(0));
					dataBytes = await decryptChunk(dataBytes, fileKey, nonceBytes);
				}
				downloadedChunks.push(dataBytes);
			}
			const blob = new Blob(downloadedChunks, { type: file.mime });
			const url = URL.createObjectURL(blob);
			const a = document.createElement('a');
			a.href = url;
			a.download = file.name;
			document.body.appendChild(a);
			a.click();
			document.body.removeChild(a);
			toast.success('Download complete!');
		} catch (e) {
			console.error(e);
			toast.error('Download failed: ' + e.message);
		} finally {
			fileToDecrypt = null;
		}
	}

	let showDeleteFileModal = $state(false);
	let fileToDelete = $state(null);

	// --- Delete Logic ---
	function handleDelete(file) {
		fileToDelete = file;
		showDeleteFileModal = true;
	}

	async function confirmDeleteFile() {
		showDeleteFileModal = false;
		if (!fileToDelete) return;

		console.log('[Delete] Requesting delete for:', fileToDelete.id, fileToDelete.name);
		try {
			console.log('[Delete] Sending API request...');
			const res = await axios.post('/api/v1/sanctum/file/delete', {
				file_id: fileToDelete.id,
				api_key: data.user.api_key
			});
			console.log('[Delete] Success response:', res);
			toast.success('File deleted');
			queryClient.invalidateQueries({ queryKey: ['fetchFiles'] });
			queryClient.invalidateQueries({ queryKey: ['fetchStorageStats'] });
		} catch (e) {
			console.error('[Delete] Failed:', e);
			toast.error('Delete failed: ' + (e.response?.data?.message || e.message));
		}
	}

	// Process API data for display
	// Process API data for display
	let recentFiles = $derived(
		fetchFiles?.data?.map((file) => ({
			...file,
			type: getFileType(file.mime)
		})) || []
	);
</script>

<div class="dashboard-container">
	<div class="dashboard-header">
		<div class="breadcrumbs-container">
			{#if folderPath.length > 1}
				<button class="icon-btn back-btn" onclick={navigateUp}>
					<Icon icon="ri:arrow-left-line" />
				</button>
			{/if}
			<div class="breadcrumbs">
				{#each folderPath as breadcrumb, i}
					{#if i > 0}
						<span class="divider">/</span>
					{/if}
					<button
						class="crumb {i === folderPath.length - 1 ? 'active' : ''} {(breadcrumb.id === null &&
							dragTargetId === 'root') ||
						(breadcrumb.id !== null && dragTargetId === breadcrumb.id)
							? 'drag-target-active-crumb'
							: ''}"
						onclick={() => navigateToBreadcrumb(i)}
						ondragover={(e) => handleBreadcrumbDragOver(e, breadcrumb)}
						ondragleave={handleItemDragLeave}
						ondrop={(e) => handleBreadcrumbDrop(e, breadcrumb)}
					>
						{breadcrumb.name}
					</button>
				{/each}
			</div>
		</div>

		<div class="header-actions">
			<button class="action-btn" onclick={() => (showUploadModal = true)}>
				<Icon icon="ri:upload-cloud-2-line" />
				<span>Upload</span>
			</button>
			<button class="action-btn outline" onclick={handleCreateFolder}>
				<Icon icon="ri:folder-add-line" />
				<span>New Folder</span>
			</button>
		</div>
	</div>

	<!-- Unified Resource Grid -->
	<div
		class="resource-grid"
		ondragover={handleDragOver}
		ondragleave={handleDragLeave}
		ondrop={handleDrop}
		role="region"
		aria-label="File drop zone"
	>
		{#if isDragging && !isInternalDrag}
			<div class="drag-overlay" transition:fade={{ duration: 150 }}>
				<Icon icon="ri:upload-cloud-2-fill" width="60" />
				<span>Drop files to upload</span>
			</div>
		{/if}

		<!-- Folders First -->
		{#each folders as folder (folder.id)}
			<div
				animate:flip={{ duration: 300 }}
				draggable="true"
				ondragstart={(e) => handleItemDragStart(e, folder, 'folder')}
				ondragover={(e) => handleItemDragOver(e, folder)}
				ondragleave={handleItemDragLeave}
				ondrop={(e) => handleItemDrop(e, folder)}
				ondragend={handleItemDragEnd}
				role="listitem"
				class={dragTargetId === folder.id ? 'drag-target-active' : ''}
				ondblclick={() => handleFolderClick(folder)}
			>
				<div class={selectedItems.has(`folder:${folder.id}`) ? 'selected-item' : ''}>
					<FolderCard
						name={folder.name}
						count={folder.count}
						starred={folder.starred}
						onclick={(e) => handleItemClick(e, folder, 'folder')}
						ondownloadzip={() => startDownloadFolderZip(folder)}
						onrename={() => handleRenameFolder(folder)}
						ondelete={() => handleDeleteFolder(folder)}
						onstar={() => handleStar(folder, 'folder')}
						onshare={() => handleShare(folder, 'folder')}
					/>
				</div>
			</div>
		{/each}

		<!-- Files Second -->
		{#each recentFiles as file (file.id)}
			<div
				animate:flip={{ duration: 300 }}
				draggable="true"
				ondragstart={(e) => handleItemDragStart(e, file, 'file')}
				ondragend={handleItemDragEnd}
				role="listitem"
			>
				<div class={selectedItems.has(`file:${file.id}`) ? 'selected-item' : ''}>
					<FileCard
						name={file.name}
						size={formatSize(file.size)}
						date={formatTime(file.created_on)}
						type={file.type}
						encrypted={file.encrypted}
						starred={file.starred}
						onclick={(e) => handleItemClick(e, file, 'file')}
						ondownload={() => handleDownload(file)}
						ondelete={() => handleDelete(file)}
						onstar={() => handleStar(file, 'file')}
						onshare={() => handleShare(file, 'file')}
					/>
				</div>
			</div>
		{/each}

		<!-- Empty State -->
		{#if folders.length === 0 && recentFiles.length === 0}
			<div class="empty-state">
				<Icon icon="ri:folder-open-line" width="64" />
				<p>This folder is empty</p>
				<button class="text-btn" onclick={() => (showUploadModal = true)}>Upload content</button>
			</div>
		{/if}
	</div>
</div>

{#if showCreateFolderModal}
	<InputModal
		bind:show={showCreateFolderModal}
		title="Create Folder"
		placeholder="Folder Name"
		submitLabel="Create"
		icon="ri:folder-add-line"
		onconfirm={confirmCreateFolder}
	/>
{/if}

{#if showRenameFolderModal}
	<InputModal
		bind:show={showRenameFolderModal}
		title="Rename Folder"
		initialValue={newFolderName}
		placeholder="New Name"
		submitLabel="Rename"
		icon="ri:edit-line"
		onconfirm={confirmRenameFolder}
	/>
{/if}

{#if showDecryptModal}
	<InputModal
		bind:show={showDecryptModal}
		title="Decrypt File"
		placeholder="Enter Password"
		submitLabel="Decrypt"
		icon="ri:lock-unlock-line"
		onconfirm={confirmDecrypt}
	/>
{/if}

{#if showDeleteFolderModal}
	<ConfirmModal
		bind:show={showDeleteFolderModal}
		title="Move Folder to Trash"
		message={`Are you sure you want to move "${folderToDelete?.name}" to trash? ${
			deletedItemCount !== null
				? deletedItemCount === 'unknown'
					? ''
					: `This includes ${deletedItemCount} items inside.`
				: '(Calculating items...)'
		}`}
		confirmLabel="Move to Trash"
		isDanger={true}
		onconfirm={confirmDeleteFolder}
	/>
{/if}

{#if showDeleteFileModal}
	<ConfirmModal
		bind:show={showDeleteFileModal}
		title="Move File to Trash"
		message={`Are you sure you want to move "${fileToDelete?.name}" to trash?`}
		confirmLabel="Move to Trash"
		isDanger={true}
		onconfirm={confirmDeleteFile}
	/>
{/if}

{#if showShareModal}
	<ShareModal item={itemToShare} on:close={() => (showShareModal = false)} />
{/if}

{#if showUploadModal}
	<div class="modal-backdrop" transition:fade={{ duration: 150 }}>
		<div class="modal-content large" transition:scale={{ duration: 200, start: 0.95 }}>
			<div class="modal-header">
				<div class="modal-title">
					<Icon icon="ri:upload-cloud-2-line" width="24" />
					<span>Upload Files</span>
				</div>
				<button class="close-btn" onclick={() => (showUploadModal = false)}>
					<Icon icon="ri:close-line" width="24" />
				</button>
			</div>

			<div class="modal-body">
				{#if !isUploading}
					<div
						class="upload-area {isDragging ? 'active' : ''}"
						ondragover={handleDragOver}
						ondragleave={handleDragLeave}
						ondrop={handleDrop}
						role="button"
						tabindex="0"
						onkeydown={(e) => e.key === 'Enter' && fileInput?.click()}
						onclick={() => fileInput?.click()}
					>
						<!-- File upload input -->
						<input
							type="file"
							bind:this={fileInput}
							hidden
							multiple
							onchange={(e) => {
								handleFileSelect(e);
								e.target.value = '';
							}}
						/>

						<Icon icon="ri:upload-cloud-2-line" width="48" class="upload-icon" />
						<p>Drag & drop files or folders here, or click to select files</p>
						<span class="sub-text" style="display: block; margin-top: 8px;"
							>Support for any file type</span
						>
					</div>

					<div class="encryption-option">
						<label class="checkbox-container">
							<input type="checkbox" bind:checked={encryptionEnabled} />
							<span class="checkmark"></span>
							<span class="label-text">Encrypt Files (Client-side)</span>
						</label>
						{#if encryptionEnabled}
							<div class="password-input" transition:fade>
								<input
									type="password"
									placeholder="Encryption Password"
									bind:value={password}
									class="input-field"
								/>
								<button class="generate-btn" onclick={() => (password = generatePassword())}>
									Generate
								</button>
							</div>
						{/if}
					</div>

					{#if files.length > 0}
						<div class="selected-files">
							<h3>Selected Files ({files.length})</h3>
							<div class="file-list-scroll">
								{#each files as file, i}
									<div class="selected-file-row">
										<Icon icon="ri:file-line" />
										<span class="name">{file.file ? file.file.name : file.name}</span>
										<span class="size">{formatSize(file.file ? file.file.size : file.size)}</span>
										<button class="remove-btn" onclick={() => removeFile(i)}>
											<Icon icon="ri:close-line" />
										</button>
									</div>
								{/each}
							</div>
						</div>
					{/if}
				{:else}
					<!-- Uploading State -->
					<div class="upload-progress-container">
						<div class="progress-circle">
							<svg viewBox="0 0 36 36" class="circular-chart">
								<path
									class="circle-bg"
									d="M18 2.0845
                                    a 15.9155 15.9155 0 0 1 0 31.831
                                    a 15.9155 15.9155 0 0 1 0 -31.831"
								/>
								<path
									class="circle"
									stroke-dasharray="{uploadStats.totalProgress}, 100"
									d="M18 2.0845
                                    a 15.9155 15.9155 0 0 1 0 31.831
                                    a 15.9155 15.9155 0 0 1 0 -31.831"
								/>
							</svg>
							<div class="percentage">{Math.round(uploadStats.totalProgress)}%</div>
						</div>
						<div class="upload-details">
							<h3>Uploading...</h3>
							<p class="current-file">{uploadStats.currentFileName}</p>
							<div class="stats-row">
								<span
									>{formatSize((uploadStats.totalBytes * uploadStats.totalProgress) / 100)} / {formatSize(
										uploadStats.totalBytes
									)}</span
								>
								<!-- <span>{uploadStats.speed} MB/s</span> -->
							</div>
						</div>
					</div>
				{/if}
			</div>

			<div class="modal-footer">
				<button
					class="btn secondary"
					onclick={() => {
						showUploadModal = false;
						files = [];
					}}>Cancel</button
				>
				<button
					class="btn primary"
					disabled={files.length === 0 || (encryptionEnabled && !password)}
					onclick={startUpload}
				>
					{encryptionEnabled ? 'Encrypt & Upload' : 'Begin Upload'}
				</button>
			</div>
		</div>
	</div>
{/if}

<style lang="scss">
	:global(body) {
		background-color: #0b0b0d;
		color: white;
		font-family: 'Outfit', sans-serif;
	}

	.dashboard-container {
		padding: 40px;
		max-width: 1600px;
		margin: 0 auto;
		display: flex;
		flex-direction: column;
		gap: 32px;
		height: 100%;
	}

	.dashboard-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 8px;

		.breadcrumbs-container {
			display: flex;
			align-items: center;
			gap: 16px;
		}

		.back-btn {
			background: rgba(255, 255, 255, 0.05);
			border: 1px solid rgba(255, 255, 255, 0.1);
			color: var(--text-primary);
			width: 36px;
			height: 36px;
			border-radius: 10px;
			display: flex;
			align-items: center;
			justify-content: center;
			cursor: pointer;
			transition: all 0.2s;

			&:hover {
				background: rgba(255, 255, 255, 0.1);
				color: white;
				border-color: rgba(255, 255, 255, 0.2);
			}
		}

		.breadcrumbs {
			display: flex;
			align-items: center;
			gap: 10px;
			font-size: 18px;
			font-weight: 500;
			color: var(--text-muted);

			.divider {
				opacity: 0.3;
				font-size: 14px;
			}

			.crumb {
				background: none;
				border: none;
				color: inherit;
				font: inherit;
				cursor: pointer;
				padding: 6px 10px;
				border-radius: 8px;
				transition: all 0.2s;

				&:hover {
					background: rgba(255, 255, 255, 0.05);
					color: var(--text-primary);
				}

				&.active {
					color: var(--text-primary);
					font-weight: 600;
					cursor: default;
					&:hover {
						background: none;
					}
				}
			}
		}

		.header-actions {
			display: flex;
			gap: 12px;

			.action-btn {
				display: flex;
				align-items: center;
				gap: 8px;
				padding: 10px 20px;
				border-radius: 10px;
				font-weight: 500;
				font-size: 14px;
				cursor: pointer;
				border: 1px solid var(--border-default);
				background: var(--bg-input);
				color: var(--text-primary);
				transition: all 0.2s;
				backdrop-filter: blur(10px);

				&:hover {
					background: var(--bg-card-hover);
					border-color: var(--border-active);
					transform: translateY(-1px);
				}

				&.outline {
					/* Same style as default now for consistency, maybe slight opacity diff */
					background: transparent;
					&:hover {
						background: var(--bg-card-hover);
					}
				}
			}
		}
	}

	.resource-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
		gap: 24px;
		position: relative;
		min-height: 200px;
		padding-bottom: 40px;

		.drag-overlay {
			position: absolute;
			inset: 0;
			background: rgba(0, 0, 0, 0.8);
			backdrop-filter: blur(8px);
			z-index: 50;
			display: flex;
			flex-direction: column;
			align-items: center;
			justify-content: center;
			gap: 16px;
			border-radius: var(--radius-lg);
			border: 2px dashed var(--primary-color);
			color: white;
			font-weight: 600;
			pointer-events: none;
		}

		.empty-state {
			grid-column: 1 / -1;
			display: flex;
			flex-direction: column;
			align-items: center;
			justify-content: center;
			padding: 80px 0;
			gap: 16px;
			color: var(--text-muted);

			p {
				font-size: 16px;
				font-weight: 500;
			}
			.text-btn {
				background: none;
				border: none;
				color: var(--primary-color);
				cursor: pointer;
				font-weight: 500;
				margin-top: 8px;
				font-size: 14px;
				opacity: 0.8;
				transition: opacity 0.2s;

				&:hover {
					opacity: 1;
					text-decoration: underline;
				}
			}
		}
	}

	/* Modal Styles */
	.modal-backdrop {
		position: fixed;
		top: 0;
		left: 0;
		width: 100vw;
		height: 100vh;
		background: rgba(0, 0, 0, 0.6);
		backdrop-filter: blur(4px);
		z-index: 1000;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 20px;
	}

	.modal-content {
		background: var(--bg-card);
		border: 1px solid var(--border-default);
		border-radius: var(--radius-lg);
		box-shadow: 0 20px 50px rgba(0, 0, 0, 0.3);
		width: 100%;
		max-width: 400px;
		display: flex;
		flex-direction: column;
		max-height: 90vh;

		&.large {
			max-width: 600px;
			height: 500px;
		}

		.modal-header {
			padding: 20px;
			border-bottom: 1px solid var(--border-default);
			display: flex;
			align-items: center;
			justify-content: space-between;

			.modal-title {
				display: flex;
				align-items: center;
				gap: 10px;
				font-weight: 600;
				font-size: 18px;
				color: var(--text-primary);
			}

			.close-btn {
				background: transparent;
				border: none;
				color: var(--text-muted);
				cursor: pointer;
				&:hover {
					color: var(--text-primary);
				}
			}
		}

		.modal-body {
			flex: 1;
			overflow-y: auto;
			padding: 20px;
		}

		.modal-footer {
			padding: 20px;
			border-top: 1px solid var(--border-default);
			display: flex;
			justify-content: flex-end;
			gap: 12px;
		}

		.btn {
			padding: 10px 20px;
			border-radius: 8px;
			font-weight: 500;
			cursor: pointer;
			border: none;
			font-size: 14px;

			&.primary {
				background: var(--primary-color);
				color: white;
				&:hover {
					opacity: 0.9;
				}
				&:disabled {
					opacity: 0.5;
					cursor: not-allowed;
				}
			}

			&.secondary {
				background: transparent;
				border: 1px solid var(--border-default);
				color: var(--text-muted);
				&:hover {
					border-color: var(--text-primary);
					color: var(--text-primary);
				}
			}
		}
	}

	.upload-area {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		height: 200px;
		border: 2px dashed var(--border-default);
		border-radius: var(--radius-md);
		cursor: pointer;
		transition: all 0.2s;
		color: var(--text-muted);
		text-align: center;
		padding: 20px;

		&:hover,
		&.active {
			border-color: var(--primary-color);
			background: rgba(234, 40, 78, 0.05);
			color: var(--primary-color);
		}

		p {
			margin: 0;
			font-weight: 500;
		}
		.sub-text {
			font-size: 12px;
			opacity: 0.7;
			margin-top: 4px;
		}
	}

	.encryption-option {
		margin-top: 24px;
		background: var(--bg-card);
		padding: 16px;
		border-radius: var(--radius-md);
		border: 1px solid var(--border-default);

		.checkbox-container {
			display: flex;
			align-items: center;
			cursor: pointer;
			user-select: none;
			position: relative;
			padding-left: 30px;

			input {
				position: absolute;
				opacity: 0;
				cursor: pointer;
				height: 0;
				width: 0;
				&:checked ~ .checkmark {
					background-color: var(--primary-color);
					border-color: var(--primary-color);
					&:after {
						display: block;
					}
				}
			}

			.checkmark {
				position: absolute;
				top: 0;
				left: 0;
				height: 20px;
				width: 20px;
				background-color: transparent;
				border: 2px solid var(--text-muted);
				border-radius: 4px;
				transition: all 0.2s;
				&:after {
					content: '';
					position: absolute;
					display: none;
					left: 6px;
					top: 2px;
					width: 5px;
					height: 10px;
					border: solid white;
					border-width: 0 2px 2px 0;
					transform: rotate(45deg);
				}
			}

			.label-text {
				color: var(--text-primary);
				font-weight: 500;
			}
		}

		.password-input {
			margin-top: 12px;
			display: flex;
			gap: 8px;

			.input-field {
				flex: 1;
				background: var(--bg-body);
				border: 1px solid var(--border-default);
				padding: 10px;
				border-radius: 6px;
				color: var(--text-primary);
				outline: none;
				&:focus {
					border-color: var(--primary-color);
				}
			}
			.generate-btn {
				padding: 0 12px;
				background: rgba(255, 255, 255, 0.1);
				border: none;
				border-radius: 6px;
				color: var(--text-primary);
				cursor: pointer;
				font-size: 13px;
				&:hover {
					background: rgba(255, 255, 255, 0.2);
				}
			}
		}
	}

	.selected-files {
		margin-top: 20px;
		h3 {
			font-size: 14px;
			color: var(--text-muted);
			margin-bottom: 12px;
		}
		.file-list-scroll {
			max-height: 120px;
			overflow-y: auto;
			display: flex;
			flex-direction: column;
			gap: 8px;
			&::-webkit-scrollbar {
				width: 4px;
			}
			&::-webkit-scrollbar-thumb {
				background: rgba(255, 255, 255, 0.1);
			}
		}
		.selected-file-row {
			display: flex;
			align-items: center;
			gap: 10px;
			background: var(--bg-input);
			padding: 8px 12px;
			border-radius: 6px;
			font-size: 14px;
			.name {
				flex: 1;
				overflow: hidden;
				text-overflow: ellipsis;
				white-space: nowrap;
				color: var(--text-primary);
			}
			.size {
				color: var(--text-muted);
				font-size: 12px;
			}
			.remove-btn {
				background: transparent;
				border: none;
				color: var(--text-muted);
				cursor: pointer;
				&:hover {
					color: #ff4655;
				}
			}
		}
	}

	.upload-progress-container {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		flex: 1;
		gap: 24px;

		.progress-circle {
			position: relative;
			width: 120px;
			height: 120px;
			.circular-chart {
				display: block;
				margin: 0 auto;
				max-width: 100%;
				max-height: 100%;
			}
			.circle-bg {
				fill: none;
				stroke: rgba(255, 255, 255, 0.05);
				stroke-width: 2.5;
			}
			.circle {
				fill: none;
				stroke: var(--primary-color);
				stroke-width: 2.5;
				stroke-linecap: round;
				transition: stroke-dasharray 0.3s ease;
			}
			.percentage {
				position: absolute;
				inset: 0;
				display: flex;
				align-items: center;
				justify-content: center;
				font-size: 24px;
				font-weight: 700;
				color: var(--text-primary);
			}
		}

		.upload-details {
			text-align: center;
			h3 {
				font-size: 18px;
				color: var(--text-primary);
				margin-bottom: 8px;
			}
			.current-file {
				color: var(--text-muted);
				font-size: 14px;
				margin-bottom: 4px;
				max-width: 300px;
				overflow: hidden;
				text-overflow: ellipsis;
				white-space: nowrap;
			}
			.stats-row {
				font-size: 13px;
				color: rgba(255, 255, 255, 0.4);
				margin-top: 8px;
			}
		}
	}

	.selected-item {
		background-color: rgba(255, 70, 85, 0.1);
		border-radius: var(--radius-md, 8px);
		border: 1px solid var(--primary-color, #ff4655);
	}

	.drag-target-active {
		position: relative;
		z-index: 10;
	}

	.drag-target-active::after {
		content: '';
		position: absolute;
		inset: -4px;
		z-index: 20;
		border-radius: 20px;
		border: 2px dashed var(--primary-color, #ff4655);
		background-color: rgba(255, 70, 85, 0.1);
		pointer-events: none;
	}

	.drag-target-active-crumb {
		outline: 2px dashed var(--primary-color, #ff4655);
		background-color: rgba(255, 70, 85, 0.1);
		border-radius: 4px;
		color: var(--primary-color) !important;
	}
</style>
