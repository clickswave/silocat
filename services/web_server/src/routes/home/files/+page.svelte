<script>
	import FolderCard from '$lib/components/FolderCard.svelte';
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
			starred: f.starred,
			created_on: f.created_on
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

	import Icon from '$lib/ui/Icon.svelte';
	import { createMutation, useQueryClient } from '@tanstack/svelte-query';
	import { toast } from '$lib/toast.js';
	import axios from 'axios';
	import {
		encryptChunk,
		deriveKeyFromPassword,
		generateSalt,
		generateNonce,
		decryptChunk
	} from '$lib/chacha.js';
	import sodium from 'libsodium-wrappers-sumo';
	import { downloadFile, fetchDecryptedBlob } from '$lib/download.js';
	import { fade, scale } from 'svelte/transition';
	import JSZip from 'jszip';

	import InputModal from '$lib/components/InputModal.svelte';
	import ConfirmModal from '$lib/components/ConfirmModal.svelte';
	import ShareModal from '$lib/components/ShareModal.svelte';

	// ... existing imports ...

	// Helper to format bytes

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

	// --- Folder Navigation State ---
	let currentFolderId = $state(null);
	let folderPath = $state([{ id: null, name: 'Files' }]);

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
		// Reset stats
		deletedItemCount = null;

		// Ideally we show a loading toast or just wait a bit?
		// Or we open modal and it shows "Calculating items..."
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
	// Cancellation for an in-flight upload. Hashing and encryption run in a
	// worker and cannot be interrupted mid-call, so the signal is checked at
	// every await boundary; the chunk PUT gets the signal directly so a large
	// transfer stops immediately rather than at the next chunk.
	let uploadAbort = $state(null);
	const UPLOAD_HALTED = 'upload-halted';

	function throwIfHalted(signal) {
		if (signal?.aborted) throw new Error(UPLOAD_HALTED);
	}

	function cancelUpload() {
		uploadAbort?.abort();
	}
	let showUploadModal = $state(false);
	let encryptionEnabled = $state(false);
	let password = $state('');
	let showPassword = $state(false);

	function copyPassword() {
		if (!password) return;
		navigator.clipboard
			.writeText(password)
			.then(() => toast.success('Password copied to clipboard'))
			.catch(() => toast.error('Could not copy password'));
	}

	// --- Multi-Select Logic ---
	let selectedItems = $state(new Set()); // Stores IDs string like "folder:123" or "file:456"
	let selectMode = $state(false); // touch-friendly: tap toggles selection
	let lastClickedKey = null; // anchor for shift-range selection

	function toggleKey(key) {
		const newSet = new Set(selectedItems);
		if (newSet.has(key)) newSet.delete(key);
		else newSet.add(key);
		selectedItems = newSet;
	}

	function selectRange(fromKey, toKey) {
		const keys = orderedKeys;
		const a = keys.indexOf(fromKey);
		const b = keys.indexOf(toKey);
		if (a === -1 || b === -1) {
			selectedItems = new Set([toKey]);
			return;
		}
		const [lo, hi] = a < b ? [a, b] : [b, a];
		selectedItems = new Set(keys.slice(lo, hi + 1));
	}

	function selectAll() {
		selectedItems = new Set(orderedKeys);
	}

	function clearSelection() {
		selectedItems = new Set();
		lastClickedKey = null;
	}

	function handleItemClick(e, item, type) {
		const key = `${type}:${item.id}`;

		if (selectMode) {
			toggleKey(key);
			lastClickedKey = key;
			return;
		}
		if (e.shiftKey && lastClickedKey) {
			selectRange(lastClickedKey, key);
			return;
		}
		if (e.ctrlKey || e.metaKey) {
			toggleKey(key);
			lastClickedKey = key;
			return;
		}
		// Plain click: single select
		selectedItems = new Set([key]);
		lastClickedKey = key;
	}

	function singleSelected() {
		if (selectedItems.size !== 1) return null;
		const [key] = [...selectedItems];
		const [type, id] = key.split(':');
		const list = type === 'folder' ? folders : recentFiles;
		const item = list.find((x) => String(x.id) === id);
		return item ? { item, type } : null;
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
		currentFileName: '',
		phase: ''
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
		mutationFn: async ({ file, folderId, onProgress, signal }) => {
			await sodium.ready; // Still need main thread sodium for random generation (nonce/salt)?
			// crypto_hash and crypto_secretbox are the heavy ones.

			throwIfHalted(signal);
			uploadStats.phase = 'Hashing file…';
			const fileChecksum = await getFileChecksum(file);
			throwIfHalted(signal);
			let key = null;
			let salt = null;

			if (encryptionEnabled) {
				if (!password) throw new Error('Password required for encrypted upload');
				salt = generateSalt(); // Fast
				uploadStats.phase = 'Deriving encryption key…';
				key = await deriveKeyFromPasswordWorker(password, salt);
				throwIfHalted(signal);
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
				throwIfHalted(signal);
				const chunkMeta = chunksMeta[i];
				const serverChunk = serverChunks[i];
				const chunkBlob = file.slice(chunkMeta.start, chunkMeta.end);
				const chunkBuffer = new Uint8Array(await chunkBlob.arrayBuffer());

				let dataToUpload = chunkBuffer;
				if (encryptionEnabled) {
					// Offload encryption to worker
					uploadStats.phase =
						totalChunks > 1 ? `Encrypting chunk ${i + 1}/${totalChunks}…` : 'Encrypting…';
					dataToUpload = await encryptChunkWorker(chunkBuffer, key, chunkMeta._rawNonce);
				}
				uploadStats.phase = 'Uploading…';

				console.log(`[Upload] Chunk ${i} URL:`, serverChunk.presigned_url);
				console.log(`[Upload] Chunk ${i} Size:`, dataToUpload.byteLength);

				const MAX_RETRIES = 3;
				let lastError;

				for (let attempt = 1; attempt <= MAX_RETRIES; attempt++) {
					try {
						await axios.put(serverChunk.presigned_url, dataToUpload, {
							signal,
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
		uploadAbort = new AbortController();
		const signal = uploadAbort.signal;
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
							throw new Error(`Failed to create folder structure: ${err.message}`);
						}
					}

					await uploadMutation.mutateAsync({
						file,
						folderId,
						signal,
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
					// A halt cancels the whole queue: the user asked to stop, not to
					// skip one file and carry on with the rest.
					if (signal.aborted || e?.message === UPLOAD_HALTED || e?.code === 'ERR_CANCELED') {
						throw new Error(UPLOAD_HALTED);
					}
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
		} catch (e) {
			if (e?.message === UPLOAD_HALTED) {
				// Files already finished stay: they are uploaded and refusing to
				// acknowledge them would be a lie. Report both halves.
				toast.info(
					successCount > 0
						? `Upload halted. ${successCount} file${successCount === 1 ? '' : 's'} already finished.`
						: 'Upload halted.'
				);
				if (successCount > 0) {
					queryClient.invalidateQueries({ queryKey: ['fetchFiles'] });
					queryClient.invalidateQueries({ queryKey: ['fetchStorageStats'] });
					queryClient.invalidateQueries({ queryKey: ['fetchFolders'] });
				}
				files = [];
				showUploadModal = false;
			} else {
				throw e;
			}
		} finally {
			isUploading = false;
			uploadAbort = null;
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
		// Progress + cancel are shown by the global DownloadToasts (bottom-right).
		fileToDecrypt = null;
		downloadFile(file, { password });
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
	let recentFiles = $derived(
		fetchFiles?.data?.map((file) => ({
			...file,
			type: getFileType(file.mime)
		})) || []
	);

	// ===================================================================
	//  View / search / sort / pagination
	// ===================================================================
	import FilePreviewModal from '$lib/components/FilePreviewModal.svelte';
	import MoveToModal from '$lib/components/MoveToModal.svelte';
	import ContextMenu from '$lib/components/ContextMenu.svelte';

	let viewMode = $state('grid'); // 'grid' | 'list'
	let searchQuery = $state('');
	let sortKey = $state('name'); // 'name' | 'size' | 'date'
	let sortDir = $state('asc'); // 'asc' | 'desc'
	const PAGE_SIZE = 60;
	let visibleCount = $state(PAGE_SIZE);

	function matchesSearch(name) {
		const q = searchQuery.trim().toLowerCase();
		return !q || (name || '').toLowerCase().includes(q);
	}

	function applySort(list, isFolder) {
		const dir = sortDir === 'asc' ? 1 : -1;
		return [...list].sort((a, b) => {
			if (sortKey === 'size') {
				const sa = isFolder ? 0 : Number(a.size) || 0;
				const sb = isFolder ? 0 : Number(b.size) || 0;
				if (sa !== sb) return dir * (sa - sb);
				return a.name.localeCompare(b.name);
			}
			if (sortKey === 'date') {
				const da = new Date(a.created_on || 0).getTime();
				const db = new Date(b.created_on || 0).getTime();
				if (da !== db) return dir * (da - db);
				return a.name.localeCompare(b.name);
			}
			return dir * a.name.localeCompare(b.name);
		});
	}

	let displayFolders = $derived(applySort(folders.filter((f) => matchesSearch(f.name)), true));
	let displayFiles = $derived(applySort(recentFiles.filter((f) => matchesSearch(f.name)), false));
	let orderedKeys = $derived([
		...displayFolders.map((f) => `folder:${f.id}`),
		...displayFiles.map((f) => `file:${f.id}`)
	]);
	let totalCount = $derived(displayFolders.length + displayFiles.length);
	let visFolders = $derived(displayFolders.slice(0, visibleCount));
	let visFiles = $derived(displayFiles.slice(0, Math.max(0, visibleCount - displayFolders.length)));
	let hasMore = $derived(totalCount > visibleCount);

	let isLoadingView = $derived(
		(fetchFiles?.isPending ?? false) || (fetchFolders?.isPending ?? false)
	);

	// Reset paging whenever the filter / sort / folder changes.
	$effect(() => {
		searchQuery;
		sortKey;
		sortDir;
		currentFolderId;
		visibleCount = PAGE_SIZE;
	});

	function toggleSort(key) {
		if (sortKey === key) sortDir = sortDir === 'asc' ? 'desc' : 'asc';
		else {
			sortKey = key;
			sortDir = 'asc';
		}
	}

	// ===================================================================
	//  Inline preview
	// ===================================================================
	let showPreview = $state(false);
	let previewFile = $state(null);
	let previewPassword = $state(null);
	let pendingPreviewFile = null;
	let showPreviewDecryptModal = $state(false);

	function isPreviewable(file) {
		const m = file?.mime || '';
		return (
			m.startsWith('image/') ||
			m.startsWith('video/') ||
			m.startsWith('audio/') ||
			m === 'application/pdf' ||
			m.startsWith('text/') ||
			m === 'application/json'
		);
	}

	function openFilePreview(file) {
		if (!isPreviewable(file)) {
			handleDownload(file);
			return;
		}
		if (file.encrypted) {
			pendingPreviewFile = file;
			showPreviewDecryptModal = true;
			return;
		}
		previewFile = file;
		previewPassword = null;
		showPreview = true;
	}

	function confirmPreviewDecrypt(pw) {
		showPreviewDecryptModal = false;
		if (!pendingPreviewFile) return;
		previewFile = pendingPreviewFile;
		previewPassword = pw;
		pendingPreviewFile = null;
		showPreview = true;
	}

	function openItem(sel) {
		if (!sel) return;
		if (sel.type === 'folder') handleFolderClick(sel.item);
		else openFilePreview(sel.item);
	}

	// ===================================================================
	//  Quick copy link
	// ===================================================================
	function copyShareLink(item) {
		const url = `${window.location.origin}/${item.id}`;
		navigator.clipboard
			.writeText(url)
			.then(() => toast.success('Link copied to clipboard'))
			.catch(() => toast.error('Could not copy link'));
	}

	// ===================================================================
	//  Bulk operations
	// ===================================================================
	function selectedList() {
		const out = [];
		for (const key of selectedItems) {
			const [type, id] = key.split(':');
			const list = type === 'folder' ? folders : recentFiles;
			const item = list.find((x) => String(x.id) === id);
			if (item) out.push({ type, item });
		}
		return out;
	}

	let bulkConfirmDelete = $state(false);
	function bulkDelete() {
		if (selectedItems.size === 0) return;
		bulkConfirmDelete = true;
	}
	async function confirmBulkDelete() {
		bulkConfirmDelete = false;
		const items = selectedList();
		let ok = 0;
		for (const { type, item } of items) {
			try {
				if (type === 'file') {
					await axios.post('/api/v1/sanctum/file/delete', {
						file_id: item.id,
						api_key: data.user.api_key
					});
				} else {
					await axios.post('/api/v1/sanctum/folder/delete', { folder_id: item.id });
				}
				ok++;
			} catch (e) {
				console.error('bulk delete', e);
			}
		}
		toast.success(`Moved ${ok} item${ok === 1 ? '' : 's'} to trash`);
		clearSelection();
		queryClient.invalidateQueries({ queryKey: ['fetchFiles'] });
		queryClient.invalidateQueries({ queryKey: ['fetchFolders'] });
		queryClient.invalidateQueries({ queryKey: ['fetchStorageStats'] });
	}

	async function bulkStar(starred) {
		const items = selectedList();
		for (const { type, item } of items) {
			try {
				await axios.post(`/api/v1/sanctum/${type}/star`, {
					[type === 'file' ? 'file_id' : 'folder_id']: item.id,
					starred
				});
			} catch (e) {
				console.error('bulk star', e);
			}
		}
		toast.success(starred ? 'Added to starred' : 'Removed from starred');
		clearSelection();
		refreshView();
		queryClient.invalidateQueries({ queryKey: ['fetchStarredFiles'] });
		queryClient.invalidateQueries({ queryKey: ['fetchStarredFolders'] });
	}

	// Bulk download (zip of selected files). Encrypted files prompt for a
	// per-file password in a modal listing each file name.
	let showBulkPwModal = $state(false);
	let bulkPwEncrypted = $state([]); // encrypted files needing a password
	let bulkPwAll = $state([]); // every selected file to include in the zip
	let bulkPwValues = $state({}); // file.id -> password

	let bulkPwReady = $derived(
		bulkPwEncrypted.length > 0 && bulkPwEncrypted.every((f) => (bulkPwValues[f.id] || '').length > 0)
	);

	function bulkDownload() {
		const fileItems = selectedList()
			.filter((s) => s.type === 'file')
			.map((s) => s.item);
		if (fileItems.length === 0) {
			toast.error('Select at least one file to download');
			return;
		}
		const encrypted = fileItems.filter((f) => f.encrypted);
		if (encrypted.length === 0) {
			doBulkZip(fileItems, {});
			return;
		}
		bulkPwAll = fileItems;
		bulkPwEncrypted = encrypted;
		bulkPwValues = Object.fromEntries(encrypted.map((f) => [f.id, '']));
		showBulkPwModal = true;
	}

	function confirmBulkPw() {
		if (!bulkPwReady) return;
		showBulkPwModal = false;
		doBulkZip(bulkPwAll, { ...bulkPwValues });
	}

	async function doBulkZip(fileItems, pwMap) {
		const toastId = toast.loading('Preparing zip download...');
		try {
			await sodium.ready;
			const zip = new JSZip();
			let okc = 0;
			const failed = [];
			for (const file of fileItems) {
				try {
					const pw = file.encrypted ? pwMap[file.id] || null : null;
					const blob = await fetchDecryptedBlob(file, { password: pw });
					zip.file(file.name, blob);
					okc++;
				} catch (e) {
					console.error('zip file failed', file.name, e);
					failed.push(file.name);
				}
			}
			if (okc === 0) throw new Error('No files could be added (wrong password?)');
			const content = await zip.generateAsync({ type: 'blob' });
			const url = URL.createObjectURL(content);
			const a = document.createElement('a');
			a.href = url;
			a.download = `silocat-${okc}-files.zip`;
			document.body.appendChild(a);
			a.click();
			document.body.removeChild(a);
			URL.revokeObjectURL(url);
			if (failed.length) {
				toast.error(
					`${failed.length} file${failed.length === 1 ? '' : 's'} skipped (wrong password?)`
				);
			}
			toast.success('Download ready');
			clearSelection();
		} catch (e) {
			toast.error('Zip failed: ' + e.message);
		} finally {
			toast.dismiss(toastId);
		}
	}

	// Bulk move
	let showMoveModal = $state(false);
	let moveExcludeIds = $derived(
		selectedList()
			.filter((s) => s.type === 'folder')
			.map((s) => s.item.id)
	);
	function bulkMoveOpen() {
		if (selectedItems.size === 0) return;
		showMoveModal = true;
	}
	async function doBulkMove(targetFolderId) {
		showMoveModal = false;
		const items = selectedList();
		let ok = 0;
		for (const { type, item } of items) {
			try {
				if (type === 'file') {
					await FrontendClient.post('/api/v1/sanctum/file/update', {
						file_id: item.id,
						new_folder_id: targetFolderId
					});
				} else {
					if (item.id === targetFolderId) continue;
					if (targetFolderId === null) {
						await FrontendClient.post('/api/v1/sanctum/folder/update', {
							folder_id: item.id,
							move_to_root: true
						});
					} else {
						await FrontendClient.post('/api/v1/sanctum/folder/update', {
							folder_id: item.id,
							new_parent_id: targetFolderId
						});
					}
				}
				ok++;
			} catch (e) {
				console.error('bulk move', e);
			}
		}
		toast.success(`Moved ${ok} item${ok === 1 ? '' : 's'}`);
		clearSelection();
		refreshView();
	}

	function bulkShareSingle() {
		const s = singleSelected();
		if (s) handleShare(s.item, s.type);
	}

	// ===================================================================
	//  Right-click context menu
	// ===================================================================
	let ctx = $state({ open: false, x: 0, y: 0, items: [] });
	function closeCtx() {
		ctx = { ...ctx, open: false };
	}
	function openItemContext(e, item, type) {
		e.preventDefault();
		e.stopPropagation();
		const key = `${type}:${item.id}`;
		if (!selectedItems.has(key)) {
			selectedItems = new Set([key]);
			lastClickedKey = key;
		}
		const multi = selectedItems.size > 1;
		const list = [];
		if (!multi) {
			if (type === 'file') {
				list.push({ label: 'Open / Preview', icon: 'ri:eye-line', action: () => openFilePreview(item) });
				list.push({ label: 'Download', icon: 'ri:download-line', action: () => handleDownload(item) });
			} else {
				list.push({ label: 'Open', icon: 'ri:folder-open-line', action: () => handleFolderClick(item) });
				list.push({ label: 'Download as zip', icon: 'ri:file-zip-line', action: () => startDownloadFolderZip(item) });
				list.push({ label: 'Rename', icon: 'ri:edit-line', action: () => handleRenameFolder(item) });
			}
			list.push({ label: 'Copy link', icon: 'ri:links-line', action: () => copyShareLink(item) });
			list.push({ label: 'Share', icon: 'ri:share-forward-line', action: () => handleShare(item, type) });
			list.push({
				label: item.starred ? 'Unstar' : 'Star',
				icon: item.starred ? 'ri:star-fill' : 'ri:star-line',
				action: () => handleStar(item, type)
			});
			list.push({ label: 'Move to…', icon: 'ri:folder-transfer-line', action: () => bulkMoveOpen() });
			list.push({ divider: true });
			list.push({
				label: 'Delete',
				icon: 'ri:delete-bin-line',
				danger: true,
				action: () => (type === 'file' ? handleDelete(item) : handleDeleteFolder(item))
			});
		} else {
			list.push({ label: `Download (${selectedItems.size})`, icon: 'ri:download-line', action: () => bulkDownload() });
			list.push({ label: 'Move to…', icon: 'ri:folder-transfer-line', action: () => bulkMoveOpen() });
			list.push({ label: 'Star', icon: 'ri:star-line', action: () => bulkStar(true) });
			list.push({ divider: true });
			list.push({ label: `Delete (${selectedItems.size})`, icon: 'ri:delete-bin-line', danger: true, action: () => bulkDelete() });
		}
		ctx = { open: true, x: e.clientX, y: e.clientY, items: list };
	}
	function openEmptyContext(e) {
		if (e.target.closest('[data-key]')) return;
		e.preventDefault();
		ctx = {
			open: true,
			x: e.clientX,
			y: e.clientY,
			items: [
				{ label: 'Upload files', icon: 'ri:upload-cloud-2-line', action: () => (showUploadModal = true) },
				{ label: 'New folder', icon: 'ri:folder-add-line', action: () => handleCreateFolder() },
				{ divider: true },
				{ label: 'Select all', icon: 'ri:checkbox-multiple-line', disabled: totalCount === 0, action: () => selectAll() }
			]
		};
	}

	// ===================================================================
	//  Keyboard shortcuts
	// ===================================================================
	function onWindowKeydown(e) {
		const t = e.target;
		const tag = t?.tagName;
		if (tag === 'INPUT' || tag === 'TEXTAREA' || t?.isContentEditable) return;
		if (showUploadModal || showCreateFolderModal || showRenameFolderModal || showDeleteFolderModal || showDeleteFileModal || showShareModal || showMoveModal || bulkConfirmDelete) {
			return;
		}
		if (showPreview) {
			if (e.key === 'Escape') showPreview = false;
			return;
		}
		if (e.key === 'Escape') {
			clearSelection();
			closeCtx();
			selectMode = false;
		} else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'a') {
			if (totalCount) {
				e.preventDefault();
				selectAll();
			}
		} else if (e.key === 'Delete' || e.key === 'Backspace') {
			if (selectedItems.size) {
				e.preventDefault();
				bulkDelete();
			}
		} else if (e.key === 'F2') {
			const s = singleSelected();
			if (s && s.type === 'folder') handleRenameFolder(s.item);
		} else if (e.key === 'Enter') {
			const s = singleSelected();
			if (s) openItem(s);
		}
	}

	// ===================================================================
	//  Marquee (drag-to-select): grid empty space
	// ===================================================================
	let gridEl = $state(null);
	let marquee = $state({ active: false, x0: 0, y0: 0, x1: 0, y1: 0 });
	let marqueeBox = $derived({
		left: Math.min(marquee.x0, marquee.x1),
		top: Math.min(marquee.y0, marquee.y1),
		width: Math.abs(marquee.x1 - marquee.x0),
		height: Math.abs(marquee.y1 - marquee.y0)
	});

	function rectsIntersect(a, b) {
		return !(a.right < b.left || a.left > b.right || a.bottom < b.top || a.top > b.bottom);
	}
	function gridMouseDown(e) {
		if (e.button !== 0) return;
		if (e.target.closest('[data-key]')) return; // on a card
		if (e.target.closest('button, a, input, .drag-overlay')) return;
		if (!(e.ctrlKey || e.metaKey || e.shiftKey)) clearSelection();
		marquee = { active: true, x0: e.clientX, y0: e.clientY, x1: e.clientX, y1: e.clientY };
		window.addEventListener('mousemove', gridMouseMove);
		window.addEventListener('mouseup', gridMouseUp);
	}
	function gridMouseMove(e) {
		if (!marquee.active) return;
		marquee = { ...marquee, x1: e.clientX, y1: e.clientY };
		const box = {
			left: Math.min(marquee.x0, marquee.x1),
			right: Math.max(marquee.x0, marquee.x1),
			top: Math.min(marquee.y0, marquee.y1),
			bottom: Math.max(marquee.y0, marquee.y1)
		};
		const sel = new Set();
		gridEl?.querySelectorAll('[data-key]').forEach((el) => {
			if (rectsIntersect(box, el.getBoundingClientRect())) sel.add(el.getAttribute('data-key'));
		});
		selectedItems = sel;
	}
	function gridMouseUp() {
		marquee = { ...marquee, active: false };
		window.removeEventListener('mousemove', gridMouseMove);
		window.removeEventListener('mouseup', gridMouseUp);
	}
	// ---- additions for the redesigned shell --------------------------------
	import { glyphForMime } from '$lib/ui/icons.js';
	import { page as pageStore } from '$app/stores';
	import { afterNavigate, replaceState } from '$app/navigation';

	// The sidebar's Upload button links to /home/files?upload=1, so arriving with
	// that flag opens the upload overlay straight away.
	//
	// afterNavigate rather than $effect, for two reasons. An effect runs during
	// hydration, before the router exists, and replaceState throws there. And an
	// effect that reads the flag re-runs when the overlay closes, sees the flag
	// still set, and reopens it, which makes Cancel and the close button look
	// broken. afterNavigate fires once per navigation, including the first, so it
	// also still works when the sidebar link is clicked from this same page.
	//
	// The flag is cleared through SvelteKit's replaceState, not the History API's:
	// the native one leaves $page.url untouched, so a later read would still see it.
	afterNavigate(() => {
		const url = new URL(window.location.href);
		if (url.searchParams.get('upload') !== '1') return;
		showUploadModal = true;
		url.searchParams.delete('upload');
		replaceState(url, {});
	});

	// Offline banner: the grid is useless without the API, and a silent empty
	// grid reads as "your files are gone", which is the worst possible lie here.
	let offline = $state(false);
	$effect(() => {
		if (!browser) return;
		offline = !navigator.onLine;
		const on = () => (offline = false);
		const off = () => (offline = true);
		window.addEventListener('online', on);
		window.addEventListener('offline', off);
		return () => {
			window.removeEventListener('online', on);
			window.removeEventListener('offline', off);
		};
	});

	function retryLoad() {
		offline = !navigator.onLine;
		refreshView();
	}

	let hasQuery = $derived(searchQuery.trim().length > 0);
	let allSelected = $derived(selectedItems.size > 0 && selectedItems.size === totalCount);
	let isEmptyState = $derived(!isLoadingView && totalCount === 0 && !hasQuery);
	let isNoResults = $derived(!isLoadingView && totalCount === 0 && hasQuery);

	function sortArrow(key) {
		if (sortKey !== key) return '';
		return sortDir === 'asc' ? ' ↑' : ' ↓';
	}

	function isSelected(type, id) {
		return selectedItems.has(`${type}:${id}`);
	}

	/** Checkbox shows in select mode, on selection, or on row hover in list view. */
	function showCheck(type, id) {
		return selectMode || isSelected(type, id);
	}
</script>

<svelte:window onkeydown={onWindowKeydown} />

<div class="files">
	<!-- Breadcrumbs + actions -->
	<div class="topbar">
		<div class="crumbs">
			{#if folderPath.length > 1}
				<button type="button" class="up" aria-label="Up one level" onclick={navigateUp}>
					<Icon name="chevron-left" size={16} />
				</button>
			{/if}
			{#each folderPath as crumb, i (crumb.id ?? 'root')}
				{#if i > 0}<span class="sep">/</span>{/if}
				{#if i === folderPath.length - 1}
					<span class="crumb current" aria-current="page">{crumb.name}</span>
				{:else}
					<button
						type="button"
						class="crumb"
						class:drop-target={(crumb.id === null && dragTargetId === 'root') ||
							(crumb.id !== null && dragTargetId === crumb.id)}
						onclick={() => navigateToBreadcrumb(i)}
						ondragover={(e) => handleBreadcrumbDragOver(e, crumb)}
						ondragleave={handleItemDragLeave}
						ondrop={(e) => handleBreadcrumbDrop(e, crumb)}
					>
						{crumb.name}
					</button>
				{/if}
			{/each}
		</div>

		<div class="top-actions">
			<button type="button" class="btn primary" onclick={() => (showUploadModal = true)}>
				<Icon name="upload" size={16} />
				Upload
			</button>
			<button type="button" class="btn ghost" onclick={handleCreateFolder}>
				<Icon name="folder-plus" size={16} />
				New Folder
			</button>
		</div>
	</div>

	<!-- Toolbar -->
	<div class="toolbar">
		<div class="search">
			<span class="search-glyph"><Icon name="search" size={15} /></span>
			<input type="text" placeholder="Search this folder" bind:value={searchQuery} spellcheck="false" />
			{#if hasQuery}
				<button
					type="button"
					class="search-clear"
					aria-label="Clear search"
					onclick={() => (searchQuery = '')}
				>
					<Icon name="close" size={13} />
				</button>
			{/if}
		</div>

		<div class="tools">
			<div class="sorts">
				<span class="sort-label">Sort</span>
				{#each [{ k: 'name', l: 'Name' }, { k: 'size', l: 'Size' }, { k: 'date', l: 'Date' }] as s (s.k)}
					<button
						type="button"
						class="sort"
						class:active={sortKey === s.k}
						onclick={() => toggleSort(s.k)}
					>
						{s.l}{sortArrow(s.k)}
					</button>
				{/each}
			</div>

			<span class="divider"></span>

			<button
				type="button"
				class="tool"
				class:on={selectMode}
				aria-label="Toggle select mode"
				aria-pressed={selectMode}
				onclick={() => {
					selectMode = !selectMode;
					if (!selectMode) clearSelection();
				}}
			>
				<Icon name="checkbox-on" size={16} />
			</button>

			<div class="view-seg">
				<button
					type="button"
					class:on={viewMode === 'grid'}
					aria-label="Grid view"
					onclick={() => (viewMode = 'grid')}
				>
					<Icon name="grid" size={15} />
				</button>
				<button
					type="button"
					class:on={viewMode === 'list'}
					aria-label="List view"
					onclick={() => (viewMode = 'list')}
				>
					<Icon name="list" size={15} />
				</button>
			</div>
		</div>
	</div>

	<!-- Bulk bar -->
	{#if selectedItems.size > 0}
		<div class="bulk" transition:fade={{ duration: 120 }}>
			<div class="bulk-left">
				<button type="button" class="bulk-x" aria-label="Clear selection" onclick={clearSelection}>
					<Icon name="close" size={14} />
				</button>
				<span class="bulk-count">{selectedItems.size} selected</span>
				<button type="button" class="bulk-link" disabled={allSelected} onclick={selectAll}>
					Select all
				</button>
			</div>
			<div class="bulk-actions">
				<button type="button" class="bulk-btn" onclick={bulkDownload}>
					<Icon name="download" size={15} />Download
				</button>
				<button type="button" class="bulk-btn" onclick={bulkMoveOpen}>
					<Icon name="folder-move" size={15} />Move
				</button>
				<button type="button" class="bulk-btn" onclick={() => bulkStar(true)}>
					<Icon name="star" size={15} />Star
				</button>
				{#if selectedItems.size === 1}
					<button type="button" class="bulk-btn" onclick={bulkShareSingle}>
						<Icon name="share" size={15} />Share
					</button>
				{/if}
				<span class="divider"></span>
				<button type="button" class="bulk-btn danger" onclick={bulkDelete}>
					<Icon name="trash" size={15} />Delete
				</button>
			</div>
		</div>
	{/if}

	{#if offline}
		<div class="offline">
			<Icon name="alert" size={15} />
			<span>You're offline. Your files are safe, we just can't reach them right now.</span>
			<button type="button" onclick={retryLoad}>Retry</button>
		</div>
	{/if}

	<!-- Resource area -->
	<!-- svelte-ignore a11y_no_static_element_interactions a11y_click_events_have_key_events -->
	<div
		class="area"
		bind:this={gridEl}
		ondragover={handleDragOver}
		ondragleave={handleDragLeave}
		ondrop={handleDrop}
		onmousedown={gridMouseDown}
		oncontextmenu={openEmptyContext}
	>
		{#if isDragging && !isInternalDrag}
			<div class="drop-overlay" transition:fade={{ duration: 120 }}>
				<Icon name="upload-lg" size={44} />
				<span>Drop files to upload</span>
			</div>
		{/if}

		{#if isLoadingView && totalCount === 0}
			<div class="grid">
				{#each Array(8) as _, i (i)}
					<div class="sk-card">
						<span class="sk-tile"></span>
						<span class="sk-line" style="width:70%"></span>
						<span class="sk-line thin" style="width:45%"></span>
					</div>
				{/each}
			</div>
		{:else if isEmptyState}
			<div class="state">
				<Icon name="folder-open" size={34} stroke={1.3} />
				<div class="state-text">
					<span class="state-title">This folder is empty</span>
					<span class="state-line">
						Drop something in, or use the button. It is encrypted before it leaves your browser.
					</span>
				</div>
				<button type="button" class="state-cta" onclick={() => (showUploadModal = true)}>
					Upload content
				</button>
			</div>
		{:else if isNoResults}
			<div class="state">
				<Icon name="search-empty" size={34} stroke={1.3} />
				<div class="state-text">
					<span class="state-title">Nothing matches “{searchQuery}”</span>
					<span class="state-line">Try a shorter search, or clear it to see everything here.</span>
				</div>
				<button type="button" class="state-cta" onclick={() => (searchQuery = '')}>
					Clear search
				</button>
			</div>
		{:else if viewMode === 'grid'}
			<div class="grid">
				{#each visFolders as folder (folder.id)}
					<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
					<div
						class="card"
						class:selected={isSelected('folder', folder.id)}
						class:drop-target={dragTargetId === folder.id}
						data-key={`folder:${folder.id}`}
						animate:flip={{ duration: 220 }}
						role="button"
						tabindex="0"
						draggable="true"
						onclick={(e) => handleItemClick(e, folder, 'folder')}
						ondblclick={() => handleFolderClick(folder)}
						onkeydown={(e) => e.key === 'Enter' && handleFolderClick(folder)}
						oncontextmenu={(e) => openItemContext(e, folder, 'folder')}
						ondragstart={(e) => handleItemDragStart(e, folder, 'folder')}
						ondragover={(e) => handleItemDragOver(e, folder)}
						ondragleave={handleItemDragLeave}
						ondrop={(e) => handleItemDrop(e, folder)}
						ondragend={handleItemDragEnd}
					>
						<div class="card-top">
							<span class="folder-glyph"><Icon name="folder-wide" size={26} stroke={1.5} /></span>
							<div class="card-tools">
								{#if folder.starred}
									<span class="ind"><Icon name="star-fill" size={14} /></span>
								{/if}
								<button
									type="button"
									class="card-btn"
									aria-label="Share"
									title="Share"
									onclick={(e) => { e.stopPropagation(); handleShare(folder, 'folder'); }}
								>
									<Icon name="share" size={14} />
								</button>
								<button
									type="button"
									class="card-btn"
									aria-label="Folder actions"
									onclick={(e) => { e.stopPropagation(); openItemContext(e, folder, 'folder'); }}
								>
									<Icon name="dots-vertical" size={15} filled />
								</button>
							</div>
						</div>
						<div class="card-text">
							<span class="card-name" title={folder.name}>{folder.name}</span>
							<span class="card-meta">{folder.count ?? 0} items</span>
						</div>
						{#if showCheck('folder', folder.id)}
							<span class="check" class:on={isSelected('folder', folder.id)}>
								<Icon name="check-sm" size={11} stroke={3} />
							</span>
						{/if}
					</div>
				{/each}

				{#each visFiles as file (file.id)}
					<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
					<div
						class="card"
						class:selected={isSelected('file', file.id)}
						data-key={`file:${file.id}`}
						animate:flip={{ duration: 220 }}
						role="button"
						tabindex="0"
						draggable="true"
						onclick={(e) => handleItemClick(e, file, 'file')}
						ondblclick={() => openFilePreview(file)}
						onkeydown={(e) => e.key === 'Enter' && openFilePreview(file)}
						oncontextmenu={(e) => openItemContext(e, file, 'file')}
						ondragstart={(e) => handleItemDragStart(e, file, 'file')}
						ondragend={handleItemDragEnd}
					>
						<div class="card-top">
							<span class="file-tile"><Icon name={glyphForMime(file.mime, file.name)} size={18} /></span>
							<div class="card-tools">
								{#if file.starred}
									<span class="ind"><Icon name="star-fill" size={14} /></span>
								{/if}
								{#if file.encrypted}
									<span class="ind faint"><Icon name="lock" size={14} stroke={1.8} /></span>
								{/if}
								<button
									type="button"
									class="card-btn"
									aria-label="Share"
									title="Share"
									onclick={(e) => { e.stopPropagation(); handleShare(file, 'file'); }}
								>
									<Icon name="share" size={14} />
								</button>
								<button
									type="button"
									class="card-btn"
									aria-label="File actions"
									onclick={(e) => { e.stopPropagation(); openItemContext(e, file, 'file'); }}
								>
									<Icon name="dots-vertical" size={15} filled />
								</button>
							</div>
						</div>
						<div class="card-text">
							<span class="card-name" title={file.name}>{file.name}</span>
							<div class="card-meta">
								<span>{formatSize(file.size)}</span>
								<span class="dot">·</span>
								<span>{formatTime(file.created_on)}</span>
							</div>
						</div>
						{#if showCheck('file', file.id)}
							<span class="check" class:on={isSelected('file', file.id)}>
								<Icon name="check-sm" size={11} stroke={3} />
							</span>
						{/if}
					</div>
				{/each}
			</div>

			{#if hasMore}
				<div class="more">
					<button type="button" onclick={() => (visibleCount += PAGE_SIZE)}>
						Load more ({totalCount - visibleCount} remaining)
					</button>
				</div>
			{/if}
		{:else}
			<div class="list">
				<div class="lhead">
					<span>Name</span>
					<span class="right">Size</span>
					<span class="right">Modified</span>
					<span></span>
				</div>

				{#each visFolders as folder (folder.id)}
					<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
					<div
						class="lrow"
						class:selected={isSelected('folder', folder.id)}
						class:drop-target={dragTargetId === folder.id}
						data-key={`folder:${folder.id}`}
						animate:flip={{ duration: 220 }}
						role="button"
						tabindex="0"
						draggable="true"
						onclick={(e) => handleItemClick(e, folder, 'folder')}
						ondblclick={() => handleFolderClick(folder)}
						onkeydown={(e) => e.key === 'Enter' && handleFolderClick(folder)}
						oncontextmenu={(e) => openItemContext(e, folder, 'folder')}
						ondragstart={(e) => handleItemDragStart(e, folder, 'folder')}
						ondragover={(e) => handleItemDragOver(e, folder)}
						ondragleave={handleItemDragLeave}
						ondrop={(e) => handleItemDrop(e, folder)}
						ondragend={handleItemDragEnd}
					>
						<div class="lname">
							{#if showCheck('folder', folder.id)}
								<span class="check inline" class:on={isSelected('folder', folder.id)}>
									<Icon name="check-sm" size={11} stroke={3} />
								</span>
							{/if}
							<span class="lglyph"><Icon name="folder" size={16} /></span>
							<span class="ltext" title={folder.name}>{folder.name}</span>
							{#if folder.starred}<span class="ind"><Icon name="star-fill" size={13} /></span>{/if}
						</div>
						<span class="lmeta right">{folder.count ?? 0} items</span>
						<span class="lmeta right">{folder.created_on ? formatTime(folder.created_on) : '-'}</span>
						<div class="lactions">
							<button
								type="button"
								class="card-btn"
								aria-label="Share"
								title="Share"
								onclick={(e) => { e.stopPropagation(); handleShare(folder, 'folder'); }}
							>
								<Icon name="share" size={14} />
							</button>
							<button
								type="button"
								class="card-btn"
								aria-label="Actions"
								onclick={(e) => { e.stopPropagation(); openItemContext(e, folder, 'folder'); }}
							>
								<Icon name="dots-vertical" size={15} filled />
							</button>
						</div>
					</div>
				{/each}

				{#each visFiles as file (file.id)}
					<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
					<div
						class="lrow"
						class:selected={isSelected('file', file.id)}
						data-key={`file:${file.id}`}
						animate:flip={{ duration: 220 }}
						role="button"
						tabindex="0"
						draggable="true"
						onclick={(e) => handleItemClick(e, file, 'file')}
						ondblclick={() => openFilePreview(file)}
						onkeydown={(e) => e.key === 'Enter' && openFilePreview(file)}
						oncontextmenu={(e) => openItemContext(e, file, 'file')}
						ondragstart={(e) => handleItemDragStart(e, file, 'file')}
						ondragend={handleItemDragEnd}
					>
						<div class="lname">
							{#if showCheck('file', file.id)}
								<span class="check inline" class:on={isSelected('file', file.id)}>
									<Icon name="check-sm" size={11} stroke={3} />
								</span>
							{/if}
							<span class="lglyph"><Icon name={glyphForMime(file.mime, file.name)} size={16} /></span>
							<span class="ltext" title={file.name}>{file.name}</span>
							{#if file.encrypted}
								<span class="ind faint"><Icon name="lock" size={13} stroke={1.9} /></span>
							{/if}
							{#if file.starred}<span class="ind"><Icon name="star-fill" size={13} /></span>{/if}
						</div>
						<span class="lmeta right">{formatSize(file.size)}</span>
						<span class="lmeta right">{formatTime(file.created_on)}</span>
						<div class="lactions">
							<button
								type="button"
								class="card-btn"
								aria-label="Share"
								title="Share"
								onclick={(e) => { e.stopPropagation(); handleShare(file, 'file'); }}
							>
								<Icon name="share" size={14} />
							</button>
							<button
								type="button"
								class="card-btn"
								aria-label="Actions"
								onclick={(e) => { e.stopPropagation(); openItemContext(e, file, 'file'); }}
							>
								<Icon name="dots-vertical" size={15} filled />
							</button>
						</div>
					</div>
				{/each}

				{#if hasMore}
					<div class="more">
						<button type="button" onclick={() => (visibleCount += PAGE_SIZE)}>
							Load more ({totalCount - visibleCount} remaining)
						</button>
					</div>
				{/if}
			</div>
		{/if}
	</div>
</div>

{#if marquee.active}
	<div
		class="marquee"
		style="left:{marqueeBox.left}px; top:{marqueeBox.top}px; width:{marqueeBox.width}px; height:{marqueeBox.height}px;"
	></div>
{/if}
{#if ctx.open}
	<ContextMenu x={ctx.x} y={ctx.y} items={ctx.items} onclose={closeCtx} />
{/if}

{#if showMoveModal}
	<MoveToModal excludeFolderIds={moveExcludeIds} onmove={doBulkMove} onclose={() => (showMoveModal = false)} />
{/if}

{#if showPreview && previewFile}
	<FilePreviewModal
		file={previewFile}
		password={previewPassword}
		onclose={() => (showPreview = false)}
		ondownload={() => {
			showPreview = false;
			handleDownload(previewFile);
		}}
	/>
{/if}

{#if showPreviewDecryptModal}
	<InputModal
		bind:show={showPreviewDecryptModal}
		title="Unlock to preview"
		label="Password"
		placeholder="Decryption password"
		hint="Set when this file was uploaded. We never had a copy."
		mono
		type="password"
		submitLabel="Unlock"
		icon="ri:lock-unlock-line"
		onconfirm={confirmPreviewDecrypt}
	/>
{/if}

{#if showBulkPwModal}
	<div
		class="modal-backdrop"
		transition:fade={{ duration: 150 }}
		role="presentation"
		onclick={(e) => {
			if (e.target === e.currentTarget) showBulkPwModal = false;
		}}
	>
		<div class="modal-content upload-modal" transition:scale={{ duration: 200, start: 0.96 }}>
			<header class="modal-header">
				<div class="modal-title">
					<Icon icon="ri:lock-2-line" width="20" />
					<span>Enter passwords</span>
				</div>
				<button class="close-btn" onclick={() => (showBulkPwModal = false)} aria-label="Close">
					<Icon icon="ri:close-line" width="22" />
				</button>
			</header>

			<div class="modal-body">
				<p class="bulkpw-intro">
					{bulkPwEncrypted.length} of the selected files {bulkPwEncrypted.length === 1
						? 'is'
						: 'are'} encrypted. Enter each password to include them in the download.
				</p>
				<div class="bulkpw-list">
					{#each bulkPwEncrypted as f (f.id)}
						<div class="bulkpw-row">
							<div class="bulkpw-file" title={f.name}>
								<Icon icon="ri:lock-fill" width="16" />
								<span>{f.name}</span>
							</div>
							<input
								type="password"
								placeholder="Password"
								autocomplete="off"
								bind:value={bulkPwValues[f.id]}
							/>
						</div>
					{/each}
				</div>
			</div>

			<footer class="modal-footer">
				<button class="btn btn-ghost" onclick={() => (showBulkPwModal = false)}>Cancel</button>
				<button class="btn btn-primary" disabled={!bulkPwReady} onclick={confirmBulkPw}>
					Download {bulkPwAll.length} file{bulkPwAll.length === 1 ? '' : 's'}
				</button>
			</footer>
		</div>
	</div>
{/if}

{#if bulkConfirmDelete}
	<ConfirmModal
		bind:show={bulkConfirmDelete}
		title="Move to Trash"
		message={`Move ${selectedItems.size} selected item${selectedItems.size === 1 ? '' : 's'} to trash?`}
		confirmLabel="Move to Trash"
		isDanger={true}
		onconfirm={confirmBulkDelete}
	/>
{/if}

{#if showCreateFolderModal}
	<InputModal
		bind:show={showCreateFolderModal}
		title="New folder"
		label="Folder name"
		placeholder="Untitled folder"
		submitLabel="Create"
		icon="ri:folder-add-line"
		onconfirm={confirmCreateFolder}
	/>
{/if}

{#if showRenameFolderModal}
	<InputModal
		bind:show={showRenameFolderModal}
		title="Rename folder"
		label="Folder name"
		initialValue={newFolderName}
		submitLabel="Rename"
		icon="ri:edit-line"
		onconfirm={confirmRenameFolder}
	/>
{/if}

{#if showDecryptModal}
	<InputModal
		bind:show={showDecryptModal}
		title="Decrypt file"
		label="Password"
		placeholder="Decryption password"
		hint="Set when this file was uploaded. We never had a copy."
		mono
		type="password"
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
	<div
		class="modal-backdrop"
		transition:fade={{ duration: 150 }}
		role="presentation"
		onclick={(e) => {
			if (e.target === e.currentTarget && !isUploading) {
				showUploadModal = false;
				files = [];
			}
		}}
	>
		<div class="modal-content upload-modal" transition:scale={{ duration: 200, start: 0.96 }}>
			<header class="modal-header">
				<div class="modal-title">
					<Icon icon="ri:upload-cloud-2-line" width="20" />
					<span>Upload files</span>
				</div>
				<!-- During an upload the cross halts it rather than closing behind a
				     transfer that keeps running unseen. -->
				<button
					class="close-btn"
					onclick={() => (isUploading ? cancelUpload() : (showUploadModal = false))}
					aria-label={isUploading ? 'Halt upload' : 'Close'}
					title={isUploading ? 'Halt upload' : 'Close'}
				>
					<Icon icon="ri:close-line" width="22" />
				</button>
			</header>

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

						<span class="upload-area-icon">
							<Icon icon="ri:upload-cloud-2-line" width="28" />
						</span>
						<p>Drop files here, or <span class="link">browse</span></p>
						<span class="sub-text">Any file type, encrypted in your browser</span>
					</div>

					<button
						type="button"
						class="encrypt-toggle {encryptionEnabled ? 'on' : ''}"
						onclick={() => (encryptionEnabled = !encryptionEnabled)}
						aria-pressed={encryptionEnabled}
					>
						<span class="encrypt-text">
							<Icon icon="ri:lock-2-line" width="18" />
							<span>
								<span class="t1">Client-side encryption</span>
								<span class="t2">Only you can unlock these files</span>
							</span>
						</span>
						<span class="switch"><span class="knob"></span></span>
					</button>

					{#if encryptionEnabled}
						<div class="password-input" transition:fade={{ duration: 120 }}>
							<input
								type={showPassword ? 'text' : 'password'}
								placeholder="Encryption password"
								bind:value={password}
								class="input-field"
							/>
							<button
								class="pw-icon-btn"
								type="button"
								title={showPassword ? 'Hide password' : 'Show password'}
								aria-label={showPassword ? 'Hide password' : 'Show password'}
								onclick={() => (showPassword = !showPassword)}
							>
								<Icon icon={showPassword ? 'ri:eye-off-line' : 'ri:eye-line'} width="18" />
							</button>
							<button
								class="pw-icon-btn"
								type="button"
								title="Copy password"
								aria-label="Copy password"
								disabled={!password}
								onclick={copyPassword}
							>
								<Icon icon="ri:file-copy-line" width="18" />
							</button>
							<button class="generate-btn" onclick={() => (password = generatePassword())}>
								Generate
							</button>
						</div>
					{/if}

					{#if files.length > 0}
						<div class="selected-files">
							<div class="selected-files-head">
								<h3>{files.length} {files.length === 1 ? 'file' : 'files'} selected</h3>
								<button class="clear-btn" onclick={() => (files = [])}>Clear all</button>
							</div>
							<div class="file-list-scroll">
								{#each files as file, i}
									<div class="selected-file-row">
										<Icon icon="ri:file-line" width="18" />
										<span class="name">{file.file ? file.file.name : file.name}</span>
										<span class="size">{formatSize(file.file ? file.file.size : file.size)}</span>
										<button
											class="remove-btn"
											aria-label="Remove file"
											onclick={() => removeFile(i)}
										>
											<Icon icon="ri:close-line" width="16" />
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
							<h3>{uploadStats.phase || 'Uploading'}</h3>
							<p class="current-file">{uploadStats.currentFileName}</p>
							<div class="stats-row">
								<span
									>{formatSize((uploadStats.totalBytes * uploadStats.totalProgress) / 100)} / {formatSize(
										uploadStats.totalBytes
									)}</span
								>
							</div>
						</div>
					</div>
				{/if}
			</div>

			{#if !isUploading}
				<footer class="modal-footer">
					<button
						class="btn btn-ghost"
						onclick={() => {
							showUploadModal = false;
							files = [];
						}}>Cancel</button
					>
					<button
						class="btn btn-primary"
						disabled={files.length === 0 || (encryptionEnabled && !password)}
						onclick={startUpload}
					>
						{encryptionEnabled ? 'Encrypt & upload' : 'Upload'}
						{#if files.length > 0}
							<span class="count-pill">{files.length}</span>
						{/if}
					</button>
				</footer>
			{/if}
		</div>
	</div>
{/if}

<style lang="scss">
	/* =====================================================================
	   Files: the core screen. Flat surfaces, one hairline, one accent.
	   The area is a single bordered card that owns its own scroll, so the
	   toolbar and bulk bar stay put while the grid moves.
	   ===================================================================== */
	.files {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
		height: 100%;
		min-height: 0;
		overflow: hidden;
	}

	/* ---- breadcrumbs + actions ---- */
	.topbar {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-4);
		padding: 0.25rem 0.125rem 0;
	}

	.crumbs {
		display: flex;
		align-items: center;
		gap: 0.375rem;
		min-width: 0;
		overflow: hidden;
	}

	.up {
		width: 28px;
		height: 28px;
		flex: 0 0 28px;
		border: 0;
		background: none;
		border-radius: var(--radius-sm);
		display: grid;
		place-items: center;
		color: var(--ink-mute);
		cursor: pointer;
		transition:
			background var(--dur-fast) var(--ease),
			color var(--dur-fast) var(--ease);

		&:hover {
			background: var(--tint-soft);
			color: var(--ink);
		}
	}

	.crumb {
		padding: 0.25rem 0.375rem;
		border: 0;
		background: none;
		border-radius: var(--radius-sm);
		font: inherit;
		font-size: 0.9375rem;
		color: var(--ink-mute);
		cursor: pointer;
		white-space: nowrap;
		transition:
			background var(--dur-fast) var(--ease),
			color var(--dur-fast) var(--ease);

		&:hover {
			background: var(--tint-soft);
			color: var(--ink);
		}
		&.current {
			font-weight: var(--fw-medium);
			color: var(--ink);
			cursor: default;
		}
		/* Breadcrumbs are drop targets: dragging onto one moves items up a level. */
		&.drop-target {
			background: var(--accent-soft);
			color: var(--accent);
		}
	}

	.sep {
		color: var(--ink-faint);
		font-size: 0.9375rem;
	}

	.top-actions {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		flex: 0 0 auto;
	}

	.btn {
		display: flex;
		align-items: center;
		gap: 0.4375rem;
		height: 34px;
		padding-inline: 0.875rem;
		border-radius: var(--radius-md);
		border: 1px solid transparent;
		font: inherit;
		font-size: var(--fs-sm);
		font-weight: var(--fw-medium);
		cursor: pointer;
		white-space: nowrap;
		transition:
			background var(--dur-fast) var(--ease),
			border-color var(--dur-fast) var(--ease);

		&.primary {
			background: var(--accent);
			color: #fff;

			&:hover {
				background: var(--accent-hover);
			}
		}
		&.ghost {
			border-color: var(--edge);
			background: none;
			color: var(--ink);

			&:hover {
				background: var(--tint-soft);
				border-color: var(--edge-strong);
			}
		}
	}

	/* ---- toolbar ---- */
	.toolbar {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-4);
		padding-inline: 0.125rem;
	}

	.search {
		position: relative;
		flex: 0 1 300px;

		input {
			width: 100%;
			height: 32px;
			padding: 0 2rem;
			border-radius: var(--radius-sm);
			background: var(--surface);
			border: 1px solid var(--edge);
			color: var(--ink);
			font: inherit;
			font-size: var(--fs-sm);
			outline: none;
			transition:
				border-color var(--dur-fast) var(--ease),
				box-shadow var(--dur-fast) var(--ease);

			&::placeholder {
				color: var(--ink-faint);
			}
			&:focus {
				border-color: var(--accent);
				box-shadow: 0 0 0 3px var(--focus-ring);
			}
		}
	}

	.search-glyph {
		position: absolute;
		left: 0.625rem;
		top: 50%;
		transform: translateY(-50%);
		color: var(--ink-faint);
		pointer-events: none;
	}

	.search-clear {
		position: absolute;
		right: 0.375rem;
		top: 50%;
		transform: translateY(-50%);
		width: 22px;
		height: 22px;
		border: 0;
		background: none;
		border-radius: 5px;
		display: grid;
		place-items: center;
		color: var(--ink-faint);
		cursor: pointer;

		&:hover {
			background: var(--tint-soft);
			color: var(--ink);
		}
	}

	.tools {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		flex: 0 0 auto;
	}

	.sorts {
		display: flex;
		align-items: center;
		gap: 0.125rem;
	}

	.sort-label {
		font-size: var(--fs-xs);
		color: var(--ink-faint);
		margin-right: var(--space-1);
	}

	.sort {
		height: 28px;
		padding-inline: var(--space-2);
		border: 0;
		background: none;
		border-radius: var(--radius-sm);
		font: inherit;
		font-size: var(--fs-sm);
		font-weight: var(--fw-medium);
		color: var(--ink-faint);
		cursor: pointer;
		white-space: nowrap;
		transition:
			background var(--dur-fast) var(--ease),
			color var(--dur-fast) var(--ease);

		&:hover {
			background: var(--tint-soft);
		}
		/* The active sort is ink, not accent: it is a state, not an action. */
		&.active {
			color: var(--ink);
		}
	}

	.divider {
		width: 1px;
		height: 18px;
		background: var(--edge);
		flex: 0 0 auto;
	}

	.tool {
		width: 30px;
		height: 30px;
		border: 0;
		background: none;
		border-radius: var(--radius-sm);
		display: grid;
		place-items: center;
		color: var(--ink-faint);
		cursor: pointer;
		transition:
			background var(--dur-fast) var(--ease),
			color var(--dur-fast) var(--ease);

		&:hover {
			background: var(--tint-soft);
			color: var(--ink);
		}
		&.on {
			background: var(--accent-soft);
			color: var(--accent);
		}
	}

	.view-seg {
		display: flex;
		padding: 2px;
		border-radius: 8px;
		background: var(--tint-soft);
		border: 1px solid var(--edge);

		button {
			width: 28px;
			height: 26px;
			border: 0;
			background: transparent;
			border-radius: var(--radius-sm);
			display: grid;
			place-items: center;
			color: var(--ink-faint);
			cursor: pointer;
			transition:
				background var(--dur-fast) var(--ease),
				color var(--dur-fast) var(--ease);

			&.on {
				background: var(--raised);
				color: var(--ink);
			}
		}
	}

	/* ---- bulk bar ---- */
	.bulk {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-4);
		height: 44px;
		padding: 0 0.625rem 0 var(--space-2);
		border-radius: var(--radius-md);
		background: var(--surface);
		border: 1px solid var(--edge-strong);
		flex: 0 0 auto;
	}

	.bulk-left {
		display: flex;
		align-items: center;
		gap: 0.625rem;
		min-width: 0;
	}

	.bulk-x {
		width: 26px;
		height: 26px;
		border: 0;
		background: none;
		border-radius: var(--radius-sm);
		display: grid;
		place-items: center;
		color: var(--ink-mute);
		cursor: pointer;

		&:hover {
			background: var(--tint-soft);
			color: var(--ink);
		}
	}

	.bulk-count {
		font-size: var(--fs-sm);
		font-weight: var(--fw-medium);
		white-space: nowrap;
	}

	.bulk-link {
		border: 0;
		background: none;
		font: inherit;
		font-size: var(--fs-sm);
		color: var(--ink-mute);
		padding: 0.25rem 0.375rem;
		border-radius: var(--radius-sm);
		cursor: pointer;
		transition: background var(--dur-fast) var(--ease);

		&:hover:not(:disabled) {
			background: var(--tint-soft);
			color: var(--ink);
		}
		&:disabled {
			color: var(--ink-faint);
			cursor: not-allowed;
		}
	}

	.bulk-actions {
		display: flex;
		align-items: center;
		gap: var(--space-1);
	}

	.bulk-btn {
		display: flex;
		align-items: center;
		gap: 0.375rem;
		height: 28px;
		padding-inline: 0.625rem;
		border: 0;
		background: none;
		border-radius: var(--radius-sm);
		font: inherit;
		font-size: var(--fs-sm);
		color: var(--ink-mute);
		cursor: pointer;
		white-space: nowrap;
		transition:
			background var(--dur-fast) var(--ease),
			color var(--dur-fast) var(--ease);

		&:hover {
			background: var(--tint-soft);
			color: var(--ink);
		}
		&.danger {
			color: var(--danger);

			&:hover {
				background: var(--danger-soft);
			}
		}
	}

	/* ---- offline ---- */
	.offline {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		padding: 0.625rem 0.875rem;
		border-radius: var(--radius-md);
		background: var(--warn-soft);
		border: 1px solid var(--edge);
		color: var(--warn);
		font-size: var(--fs-sm);
		flex: 0 0 auto;

		span {
			flex: 1;
			color: var(--ink-mute);
		}

		button {
			border: 0;
			background: none;
			font: inherit;
			font-size: var(--fs-sm);
			font-weight: var(--fw-medium);
			color: var(--warn);
			cursor: pointer;
		}
	}

	/* ---- area ---- */
	.area {
		position: relative;
		flex: 1;
		min-height: 0;
		border: 1px solid var(--edge);
		border-radius: var(--radius-md);
		background: var(--surface);
		overflow: auto;
	}

	.drop-overlay {
		position: absolute;
		inset: 0;
		z-index: 5;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: var(--space-3);
		border-radius: var(--radius-md);
		border: 1px dashed var(--accent);
		background: var(--accent-soft);
		color: var(--accent);
		font-size: var(--fs-body);
		font-weight: var(--fw-medium);
		pointer-events: none;
	}

	.grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(190px, 1fr));
		gap: 0.75rem;
		padding: 1rem;
	}

	/* ---- cards ---- */
	.card {
		position: relative;
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
		/* The corner checkbox lives in this bottom gutter. It is reserved on
		   every card, selected or not, so selecting one never reflows the grid. */
		padding: 0.875rem 0.875rem 2.25rem;
		border-radius: var(--radius-md);
		/* Cards carry their hairline at rest, so the grid reads as a set of
		   objects rather than floating text; selection swaps it to accent. */
		border: 1px solid var(--edge);
		background: transparent;
		cursor: default;
		transition:
			background var(--dur-fast) var(--ease),
			border-color var(--dur-fast) var(--ease);

		&:hover {
			background: var(--surface-hover);

			.card-btn {
				opacity: 1;
			}
		}
		&.selected {
			background: var(--accent-soft);
			border-color: var(--accent);
		}
		&.drop-target {
			border-color: var(--accent);
			background: var(--accent-soft);
		}
	}

	.card-top {
		display: flex;
		align-items: flex-start;
		justify-content: space-between;
	}

	.folder-glyph {
		color: var(--ink-mute);
		display: grid;
		place-items: center;
	}

	.file-tile {
		display: grid;
		place-items: center;
		width: 34px;
		height: 34px;
		border-radius: 8px;
		background: var(--tint-soft);
		color: var(--ink-mute);
	}

	.card-tools {
		display: flex;
		align-items: center;
		gap: 0.375rem;
	}

	.ind {
		display: grid;
		place-items: center;
		color: var(--ink-mute);

		&.faint {
			color: var(--ink-faint);
		}
	}

	/* Hover-revealed on cards, always visible in list rows. */
	.card-btn {
		width: 24px;
		height: 24px;
		border: 0;
		background: none;
		border-radius: var(--radius-sm);
		display: grid;
		place-items: center;
		color: var(--ink-faint);
		cursor: pointer;
		/* Visible but quiet at rest: hiding them entirely means people never
		   learn the card has a share button. */
		opacity: 0.55;
		transition:
			opacity var(--dur-fast) var(--ease),
			background var(--dur-fast) var(--ease),
			color var(--dur-fast) var(--ease);

		&:hover,
		&:focus-visible {
			background: var(--tint-softer);
			color: var(--ink);
			opacity: 1;
		}
	}

	.card-text {
		display: flex;
		flex-direction: column;
		gap: 0.1875rem;
		min-width: 0;
	}

	.card-name {
		font-size: var(--fs-sm);
		font-weight: var(--fw-medium);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.card-meta {
		display: flex;
		align-items: center;
		gap: 0.4375rem;
		font-family: var(--font-mono);
		font-size: var(--fs-xs);
		color: var(--ink-faint);
	}

	.dot {
		opacity: 0.5;
	}

	.check {
		position: absolute;
		left: 0.625rem;
		bottom: 0.625rem;
		width: 16px;
		height: 16px;
		border-radius: 4px;
		display: grid;
		place-items: center;
		background: var(--surface);
		border: 1px solid var(--edge-strong);
		color: #fff;

		:global(svg) {
			opacity: 0;
		}

		&.on {
			background: var(--accent);
			border-color: var(--accent);

			:global(svg) {
				opacity: 1;
			}
		}
		&.inline {
			position: static;
			flex: 0 0 16px;
		}
	}

	/* ---- list ---- */
	.lhead,
	.lrow {
		display: grid;
		grid-template-columns: 1fr 110px 130px 64px;
		gap: var(--space-4);
		align-items: center;
	}

	.lhead {
		padding: 0.625rem 1rem;
		border-bottom: 1px solid var(--edge);
		font-size: var(--fs-xs);
		color: var(--ink-faint);
		position: sticky;
		top: 0;
		background: var(--surface);
		z-index: 1;
	}

	.lrow {
		padding: 0.5rem 1rem;
		border-bottom: 1px solid var(--edge);
		cursor: default;
		transition: background var(--dur-fast) var(--ease);

		&:hover {
			background: var(--surface-hover);
		}
		&.selected {
			background: var(--accent-soft);
			box-shadow: inset 2px 0 0 var(--accent);
		}
		&.drop-target {
			background: var(--accent-soft);
			box-shadow: inset 0 0 0 1px var(--accent);
		}
	}

	.lname {
		display: flex;
		align-items: center;
		gap: 0.625rem;
		min-width: 0;
	}

	.lglyph {
		flex: 0 0 auto;
		display: grid;
		place-items: center;
		color: var(--ink-mute);
	}

	.ltext {
		font-size: var(--fs-sm);
		font-weight: var(--fw-medium);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.lmeta {
		font-family: var(--font-mono);
		font-size: var(--fs-xs);
		color: var(--ink-faint);
	}

	.right {
		text-align: right;
	}

	.lactions {
		display: flex;
		align-items: center;
		gap: 0.125rem;
		justify-self: end;

		.card-btn {
			opacity: 1;
			width: 26px;
			height: 26px;
		}
	}

	/* ---- states ---- */
	.state {
		height: 100%;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 0.875rem;
		padding: 3rem 1rem;
		text-align: center;
		color: var(--ink-faint);
	}

	.state-text {
		display: flex;
		flex-direction: column;
		gap: 0.375rem;
		max-width: 34ch;
	}

	.state-title {
		font-size: var(--fs-lg);
		font-weight: var(--fw-medium);
		letter-spacing: var(--tracking-tight);
		color: var(--ink);
	}

	.state-line {
		font-size: var(--fs-sm);
		color: var(--ink-mute);
		line-height: var(--lh-normal);
	}

	.state-cta {
		border: 0;
		background: none;
		font: inherit;
		font-size: var(--fs-sm);
		font-weight: var(--fw-medium);
		color: var(--accent);
		padding: 0.4375rem 0.625rem;
		border-radius: var(--radius-sm);
		cursor: pointer;
		transition: background var(--dur-fast) var(--ease);

		&:hover {
			background: var(--accent-soft);
		}
	}

	.more {
		display: flex;
		justify-content: center;
		padding: 0 1rem 1.25rem;

		button {
			border: 0;
			background: none;
			font: inherit;
			font-size: var(--fs-sm);
			color: var(--ink-mute);
			padding: 0.4375rem 0.75rem;
			border-radius: var(--radius-sm);
			cursor: pointer;
			transition:
				background var(--dur-fast) var(--ease),
				color var(--dur-fast) var(--ease);

			&:hover {
				background: var(--tint-soft);
				color: var(--ink);
			}
		}
	}

	/* ---- skeletons ---- */
	.sk-card {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
		padding: 0.875rem;
		border-radius: var(--radius-md);
		border: 1px solid var(--edge);
		animation: sk-pulse 1.4s ease-in-out infinite;
	}

	.sk-tile {
		width: 34px;
		height: 34px;
		border-radius: 8px;
		background: var(--tint-softer);
	}

	.sk-line {
		height: 10px;
		border-radius: 4px;
		background: var(--tint-softer);

		&.thin {
			height: 9px;
			background: var(--tint-soft);
		}
	}

	@keyframes sk-pulse {
		0%,
		100% {
			opacity: 1;
		}
		50% {
			opacity: 0.55;
		}
	}

	/* ---- marquee ---- */
	.marquee {
		position: fixed;
		z-index: 40;
		border: 1px solid var(--accent);
		background: var(--accent-soft);
		border-radius: 4px;
		pointer-events: none;
	}

	/* =====================================================================
	   Overlays that are still hand-rolled in this file (upload, bulk
	   passwords). Same shell tokens as `ui/Modal`, so they read identically.
	   ===================================================================== */
	.modal-backdrop {
		position: fixed;
		inset: 0;
		z-index: 1000;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: var(--space-5);
		background: var(--scrim);
	}

	.modal-content {
		width: 100%;
		max-width: 480px;
		display: flex;
		flex-direction: column;
		background: var(--raised);
		border: 1px solid var(--edge);
		border-radius: var(--radius-lg);
		box-shadow: var(--shadow-overlay);
		overflow: hidden;
		max-height: min(84vh, 760px);
	}

	.modal-header {
		display: flex;
		align-items: center;
		gap: 0.625rem;
		padding: 1rem 1rem 0.875rem;
	}

	.modal-title {
		display: flex;
		align-items: center;
		gap: 0.625rem;
		flex: 1;
		min-width: 0;
		font-size: 0.9375rem;
		font-weight: var(--fw-semibold);
		letter-spacing: var(--tracking-tight);
		color: var(--ink);
	}

	.close-btn {
		width: 28px;
		height: 28px;
		border: 0;
		background: none;
		border-radius: var(--radius-sm);
		display: grid;
		place-items: center;
		color: var(--ink-faint);
		cursor: pointer;

		&:hover {
			background: var(--tint-soft);
			color: var(--ink);
		}
	}

	.modal-body {
		display: flex;
		flex-direction: column;
		gap: 0.875rem;
		padding: 0 1rem 1rem;
		overflow-y: auto;
	}

	.modal-footer {
		display: flex;
		align-items: center;
		justify-content: flex-end;
		gap: var(--space-2);
		padding: 0.875rem 1rem;
		border-top: 1px solid var(--edge);
	}

	.upload-area {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: var(--space-2);
		padding: 2rem 1rem;
		border: 1px dashed var(--edge-strong);
		border-radius: var(--radius-md);
		background: var(--bg);
		color: var(--ink-faint);
		cursor: pointer;
		text-align: center;
		transition:
			border-color var(--dur-fast) var(--ease),
			background var(--dur-fast) var(--ease);

		&:hover,
		&.active {
			border-color: var(--accent);
			background: var(--accent-soft);
		}

		p {
			margin: 0;
			font-size: var(--fs-body);
			color: var(--ink);
		}

		.link {
			color: var(--accent);
		}

		.sub-text {
			font-size: var(--fs-sm);
			color: var(--ink-faint);
		}
	}

	.upload-area-icon {
		display: grid;
		place-items: center;
		color: var(--ink-faint);
	}

	.encrypt-toggle {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-3);
		padding: 0.75rem;
		border: 1px solid var(--edge);
		border-radius: var(--radius-md);
		background: none;
		font: inherit;
		color: var(--ink);
		cursor: pointer;
		text-align: left;

		.encrypt-text {
			display: flex;
			align-items: center;
			gap: 0.625rem;
			color: var(--ink-mute);

			span {
				display: flex;
				flex-direction: column;
			}
		}
		.t1 {
			font-size: 0.875rem;
			font-weight: var(--fw-medium);
			color: var(--ink);
		}
		.t2 {
			font-size: var(--fs-xs);
			color: var(--ink-faint);
		}

		.switch {
			position: relative;
			flex: 0 0 auto;
			width: 34px;
			height: 20px;
			border-radius: var(--radius-full);
			background: var(--tint-softer);
			transition: background var(--dur-fast) var(--ease);
		}
		.knob {
			position: absolute;
			top: 2px;
			left: 2px;
			width: 16px;
			height: 16px;
			border-radius: var(--radius-full);
			background: #fff;
			transition: left var(--dur-fast) var(--ease);
		}
		&.on .switch {
			background: var(--accent);
		}
		&.on .knob {
			left: 16px;
		}
	}

	.password-input {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		height: 38px;
		padding-inline: 0.75rem;
		border-radius: var(--radius-sm);
		background: var(--bg);
		border: 1px solid var(--edge);

		.input-field {
			flex: 1;
			min-width: 0;
			border: 0;
			background: none;
			outline: none;
			color: var(--ink);
			font-family: var(--font-mono);
			font-size: 0.875rem;
		}
	}

	.pw-icon-btn {
		border: 0;
		background: none;
		color: var(--ink-faint);
		cursor: pointer;
		display: grid;
		place-items: center;

		&:hover:not(:disabled) {
			color: var(--ink);
		}
		&:disabled {
			opacity: 0.4;
			cursor: not-allowed;
		}
	}

	.generate-btn {
		flex: 0 0 auto;
		height: 26px;
		padding-inline: 0.5rem;
		border-radius: var(--radius-sm);
		border: 1px solid var(--edge);
		background: none;
		font: inherit;
		font-size: var(--fs-xs);
		font-weight: var(--fw-medium);
		color: var(--ink);
		cursor: pointer;

		&:hover {
			background: var(--tint-soft);
		}
	}

	.selected-files {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		border: 1px solid var(--edge);
		border-radius: var(--radius-md);
		overflow: hidden;
	}

	.selected-files-head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 0.625rem 0.75rem;
		border-bottom: 1px solid var(--edge);

		h3 {
			margin: 0;
			font-size: var(--fs-sm);
			font-weight: var(--fw-medium);
		}
	}

	.clear-btn {
		border: 0;
		background: none;
		font: inherit;
		font-size: var(--fs-xs);
		color: var(--ink-mute);
		cursor: pointer;

		&:hover {
			color: var(--ink);
		}
	}

	.file-list-scroll {
		max-height: 180px;
		overflow-y: auto;
	}

	.selected-file-row {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		padding: 0.5rem 0.75rem;
		color: var(--ink-faint);

		& + .selected-file-row {
			border-top: 1px solid var(--edge);
		}

		.name {
			flex: 1;
			min-width: 0;
			font-size: var(--fs-sm);
			color: var(--ink);
			overflow: hidden;
			text-overflow: ellipsis;
			white-space: nowrap;
		}
		.size {
			font-family: var(--font-mono);
			font-size: var(--fs-xs);
		}
	}

	.remove-btn {
		width: 22px;
		height: 22px;
		border: 0;
		background: none;
		border-radius: var(--radius-sm);
		display: grid;
		place-items: center;
		color: var(--ink-faint);
		cursor: pointer;

		&:hover {
			background: var(--tint-soft);
			color: var(--ink);
		}
	}

	/* Uploading state: one ring, one phase line, mono bytes. */
	.upload-progress-container {
		display: flex;
		align-items: center;
		gap: 1.25rem;
		padding: 0.5rem 0;
	}

	.progress-circle {
		position: relative;
		flex: 0 0 96px;
		width: 96px;
		height: 96px;
	}

	.circular-chart {
		width: 96px;
		height: 96px;
		transform: rotate(-90deg);
	}

	.circle-bg {
		fill: none;
		stroke: var(--tint-softer);
		stroke-width: 2.4;
	}

	.circle {
		fill: none;
		stroke: var(--accent);
		stroke-width: 2.4;
		stroke-linecap: round;
		transition: stroke-dasharray var(--dur) var(--ease);
	}

	.percentage {
		position: absolute;
		inset: 0;
		display: grid;
		place-items: center;
		font-family: var(--font-mono);
		font-size: var(--fs-body);
		color: var(--ink);
	}

	.upload-details {
		display: flex;
		flex-direction: column;
		gap: 0.375rem;
		min-width: 0;

		h3 {
			margin: 0;
			font-size: var(--fs-body);
			font-weight: var(--fw-medium);
		}
	}

	.current-file {
		margin: 0;
		font-size: var(--fs-sm);
		color: var(--ink-mute);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.stats-row {
		font-family: var(--font-mono);
		font-size: var(--fs-xs);
		color: var(--ink-faint);
	}

	.bulkpw-intro {
		margin: 0;
		font-size: var(--fs-sm);
		color: var(--ink-mute);
	}

	.bulkpw-list {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		max-height: 260px;
		overflow-y: auto;
	}

	.bulkpw-row {
		display: flex;
		align-items: center;
		gap: var(--space-2);

		input {
			flex: 0 0 180px;
			height: 32px;
			padding-inline: 0.625rem;
			border-radius: var(--radius-sm);
			background: var(--bg);
			border: 1px solid var(--edge);
			color: var(--ink);
			font-family: var(--font-mono);
			font-size: var(--fs-sm);
			outline: none;

			&:focus {
				border-color: var(--accent);
				box-shadow: 0 0 0 3px var(--focus-ring);
			}
		}
	}

	.bulkpw-file {
		flex: 1;
		min-width: 0;
		display: flex;
		align-items: center;
		gap: var(--space-2);
		font-size: var(--fs-sm);
		color: var(--ink-mute);

		span {
			overflow: hidden;
			text-overflow: ellipsis;
			white-space: nowrap;
		}
	}

	.btn-ghost,
	.btn-primary {
		height: 34px;
		padding-inline: 0.875rem;
		border-radius: var(--radius-md);
		border: 0;
		font: inherit;
		font-size: var(--fs-sm);
		font-weight: var(--fw-medium);
		cursor: pointer;
		display: inline-flex;
		align-items: center;
		gap: var(--space-2);
		transition:
			background var(--dur-fast) var(--ease),
			color var(--dur-fast) var(--ease);
	}

	.btn-ghost {
		background: none;
		color: var(--ink-mute);

		&:hover {
			background: var(--tint-soft);
			color: var(--ink);
		}
	}

	.btn-primary {
		background: var(--accent);
		color: #fff;

		&:hover:not(:disabled) {
			background: var(--accent-hover);
		}
		&:disabled {
			opacity: 0.5;
			cursor: not-allowed;
		}
	}

	.count-pill {
		display: inline-grid;
		place-items: center;
		min-width: 18px;
		height: 18px;
		padding-inline: 5px;
		border-radius: var(--radius-full);
		background: rgba(255, 255, 255, 0.22);
		font-family: var(--font-mono);
		font-size: 0.6875rem;
	}

	@media (max-width: 900px) {
		.toolbar {
			flex-wrap: wrap;
		}
		.search {
			flex: 1 1 100%;
		}
		.sort-label {
			display: none;
		}
		.lhead,
		.lrow {
			grid-template-columns: 1fr 90px 64px;
		}
		.lhead span:nth-child(3),
		.lrow > .lmeta:nth-of-type(2) {
			display: none;
		}
	}

	@media (max-width: 640px) {
		.bulk {
			height: auto;
			flex-direction: column;
			align-items: stretch;
			gap: var(--space-2);
			padding: var(--space-2);
		}
		.bulk-actions {
			flex-wrap: wrap;
		}
	}
</style>
