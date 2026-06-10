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
	import { downloadFile, fetchDecryptedBlob } from '$lib/download.js';
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
		mutationFn: async ({ file, folderId, onProgress }) => {
			await sodium.ready; // Still need main thread sodium for random generation (nonce/salt)?
			// Actually, random generation is fast and non-blocking.
			// crypto_hash and crypto_secretbox are the heavy ones.

			uploadStats.phase = 'Hashing file…';
			const fileChecksum = await getFileChecksum(file);
			let key = null;
			let salt = null;

			if (encryptionEnabled) {
				if (!password) throw new Error('Password required for encrypted upload');
				salt = generateSalt(); // Fast
				uploadStats.phase = 'Deriving encryption key…';
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
	//  Marquee (drag-to-select) — grid empty space
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
</script>

<svelte:window onkeydown={onWindowKeydown} />

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

	<!-- Toolbar: search / sort / view / select -->
	<div class="files-toolbar">
		<div class="search-box">
			<Icon icon="ri:search-line" width="18" />
			<input
				type="text"
				placeholder="Search this folder"
				bind:value={searchQuery}
				spellcheck="false"
			/>
			{#if searchQuery}
				<button class="clear-search" aria-label="Clear search" onclick={() => (searchQuery = '')}>
					<Icon icon="ri:close-line" width="16" />
				</button>
			{/if}
		</div>

		<div class="toolbar-right">
			<div class="sort-group">
				{#each [{ k: 'name', l: 'Name' }, { k: 'size', l: 'Size' }, { k: 'date', l: 'Date' }] as s}
					<button
						class="sort-btn {sortKey === s.k ? 'active' : ''}"
						onclick={() => toggleSort(s.k)}
					>
						{s.l}
						{#if sortKey === s.k}
							<Icon icon={sortDir === 'asc' ? 'ri:arrow-up-s-line' : 'ri:arrow-down-s-line'} width="16" />
						{/if}
					</button>
				{/each}
			</div>

			<button
				class="tool-icon {selectMode ? 'active' : ''}"
				title="Select mode"
				aria-label="Toggle select mode"
				onclick={() => {
					selectMode = !selectMode;
					if (!selectMode) clearSelection();
				}}
			>
				<Icon icon="ri:checkbox-multiple-line" width="18" />
			</button>

			<div class="view-toggle">
				<button
					class={viewMode === 'grid' ? 'active' : ''}
					title="Grid view"
					aria-label="Grid view"
					onclick={() => (viewMode = 'grid')}
				>
					<Icon icon="ri:layout-grid-line" width="18" />
				</button>
				<button
					class={viewMode === 'list' ? 'active' : ''}
					title="List view"
					aria-label="List view"
					onclick={() => (viewMode = 'list')}
				>
					<Icon icon="ri:list-unordered" width="18" />
				</button>
			</div>
		</div>
	</div>

	<!-- Bulk action bar -->
	{#if selectedItems.size > 0}
		<div class="bulk-bar" transition:fade={{ duration: 120 }}>
			<div class="bulk-left">
				<button class="bulk-x" aria-label="Clear selection" onclick={clearSelection}>
					<Icon icon="ri:close-line" width="18" />
				</button>
				<span class="bulk-count">{selectedItems.size} selected</span>
				<button class="bulk-link" onclick={selectAll} disabled={selectedItems.size === totalCount}>
					Select all
				</button>
			</div>
			<div class="bulk-actions">
				<button class="bulk-btn" onclick={bulkDownload} title="Download">
					<Icon icon="ri:download-line" width="18" /><span>Download</span>
				</button>
				<button class="bulk-btn" onclick={bulkMoveOpen} title="Move">
					<Icon icon="ri:folder-transfer-line" width="18" /><span>Move</span>
				</button>
				<button class="bulk-btn" onclick={() => bulkStar(true)} title="Star">
					<Icon icon="ri:star-line" width="18" /><span>Star</span>
				</button>
				{#if selectedItems.size === 1}
					<button class="bulk-btn" onclick={bulkShareSingle} title="Share">
						<Icon icon="ri:share-forward-line" width="18" /><span>Share</span>
					</button>
				{/if}
				<button class="bulk-btn danger" onclick={bulkDelete} title="Delete">
					<Icon icon="ri:delete-bin-line" width="18" /><span>Delete</span>
				</button>
			</div>
		</div>
	{/if}

	<!-- Unified Resource Area -->
	<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
	<div
		class="resource-area {viewMode}"
		bind:this={gridEl}
		ondragover={handleDragOver}
		ondragleave={handleDragLeave}
		ondrop={handleDrop}
		onmousedown={gridMouseDown}
		oncontextmenu={openEmptyContext}
		role="region"
		aria-label="File drop zone"
	>
		{#if isDragging && !isInternalDrag}
			<div class="drag-overlay" transition:fade={{ duration: 150 }}>
				<Icon icon="ri:upload-cloud-2-fill" width="60" />
				<span>Drop files to upload</span>
			</div>
		{/if}

		{#if isLoadingView && totalCount === 0}
			<!-- Skeletons -->
			{#each Array(viewMode === 'grid' ? 10 : 8) as _, i (i)}
				<div class="skeleton {viewMode}"></div>
			{/each}
		{:else if totalCount === 0}
			<!-- Empty / no-results state -->
			<div class="empty-state">
				{#if searchQuery}
					<Icon icon="ri:search-eye-line" width="64" />
					<p>No items match “{searchQuery}”.</p>
					<button class="text-btn" onclick={() => (searchQuery = '')}>Clear search</button>
				{:else}
					<Icon icon="ri:folder-open-line" width="64" />
					<p>This folder is empty</p>
					<button class="text-btn" onclick={() => (showUploadModal = true)}>Upload content</button>
				{/if}
			</div>
		{:else if viewMode === 'grid'}
			<!-- GRID: folders then files -->
			{#each visFolders as folder (folder.id)}
				<div
					class="cell {dragTargetId === folder.id ? 'drag-target-active' : ''} {selectedItems.has(`folder:${folder.id}`) ? 'is-selected' : ''}"
					data-key={`folder:${folder.id}`}
					animate:flip={{ duration: 250 }}
					draggable="true"
					ondragstart={(e) => handleItemDragStart(e, folder, 'folder')}
					ondragover={(e) => handleItemDragOver(e, folder)}
					ondragleave={handleItemDragLeave}
					ondrop={(e) => handleItemDrop(e, folder)}
					ondragend={handleItemDragEnd}
					oncontextmenu={(e) => openItemContext(e, folder, 'folder')}
					ondblclick={() => handleFolderClick(folder)}
					role="listitem"
				>
					{#if selectMode || selectedItems.has(`folder:${folder.id}`)}
						<button
							class="sel-check {selectedItems.has(`folder:${folder.id}`) ? 'on' : ''}"
							aria-label="Select"
							onclick={(e) => {
								e.stopPropagation();
								toggleKey(`folder:${folder.id}`);
							}}
						>
							<Icon icon={selectedItems.has(`folder:${folder.id}`) ? 'ri:checkbox-fill' : 'ri:checkbox-blank-line'} width="20" />
						</button>
					{/if}
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
			{/each}

			{#each visFiles as file (file.id)}
				<div
					class="cell {selectedItems.has(`file:${file.id}`) ? 'is-selected' : ''}"
					data-key={`file:${file.id}`}
					animate:flip={{ duration: 250 }}
					draggable="true"
					ondragstart={(e) => handleItemDragStart(e, file, 'file')}
					ondragend={handleItemDragEnd}
					oncontextmenu={(e) => openItemContext(e, file, 'file')}
					ondblclick={() => openFilePreview(file)}
					role="listitem"
				>
					{#if selectMode || selectedItems.has(`file:${file.id}`)}
						<button
							class="sel-check {selectedItems.has(`file:${file.id}`) ? 'on' : ''}"
							aria-label="Select"
							onclick={(e) => {
								e.stopPropagation();
								toggleKey(`file:${file.id}`);
							}}
						>
							<Icon icon={selectedItems.has(`file:${file.id}`) ? 'ri:checkbox-fill' : 'ri:checkbox-blank-line'} width="20" />
						</button>
					{/if}
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
			{/each}
		{:else}
			<!-- LIST VIEW -->
			<div class="list-head">
				<span class="lh-name">Name</span>
				<span class="lh-size">Size</span>
				<span class="lh-date">Modified</span>
				<span class="lh-actions"></span>
			</div>

			{#each visFolders as folder (folder.id)}
				<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_noninteractive_element_interactions -->
				<div
					class="row {dragTargetId === folder.id ? 'drag-target-active' : ''} {selectedItems.has(`folder:${folder.id}`) ? 'is-selected' : ''}"
					data-key={`folder:${folder.id}`}
					animate:flip={{ duration: 250 }}
					draggable="true"
					ondragstart={(e) => handleItemDragStart(e, folder, 'folder')}
					ondragover={(e) => handleItemDragOver(e, folder)}
					ondragleave={handleItemDragLeave}
					ondrop={(e) => handleItemDrop(e, folder)}
					ondragend={handleItemDragEnd}
					oncontextmenu={(e) => openItemContext(e, folder, 'folder')}
					onclick={(e) => handleItemClick(e, folder, 'folder')}
					ondblclick={() => handleFolderClick(folder)}
					role="listitem"
				>
					<span class="r-name">
						{#if selectMode || selectedItems.has(`folder:${folder.id}`)}
							<button
								class="sel-check inline {selectedItems.has(`folder:${folder.id}`) ? 'on' : ''}"
								aria-label="Select"
								onclick={(e) => { e.stopPropagation(); toggleKey(`folder:${folder.id}`); }}
							>
								<Icon icon={selectedItems.has(`folder:${folder.id}`) ? 'ri:checkbox-fill' : 'ri:checkbox-blank-line'} width="18" />
							</button>
						{/if}
						<Icon icon="ri:folder-3-fill" width="20" class="r-icon folder" />
						<span class="nm" title={folder.name}>{folder.name}</span>
						{#if folder.starred}<Icon icon="ri:star-fill" width="14" class="r-star" />{/if}
					</span>
					<span class="r-size">{folder.count} items</span>
					<span class="r-date">{folder.created_on ? formatTime(folder.created_on) : '—'}</span>
					<button
						class="r-menu"
						aria-label="Actions"
						onclick={(e) => { e.stopPropagation(); openItemContext(e, folder, 'folder'); }}
					>
						<Icon icon="ri:more-2-fill" width="18" />
					</button>
				</div>
			{/each}

			{#each visFiles as file (file.id)}
				<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_noninteractive_element_interactions -->
				<div
					class="row {selectedItems.has(`file:${file.id}`) ? 'is-selected' : ''}"
					data-key={`file:${file.id}`}
					animate:flip={{ duration: 250 }}
					draggable="true"
					ondragstart={(e) => handleItemDragStart(e, file, 'file')}
					ondragend={handleItemDragEnd}
					oncontextmenu={(e) => openItemContext(e, file, 'file')}
					onclick={(e) => handleItemClick(e, file, 'file')}
					ondblclick={() => openFilePreview(file)}
					role="listitem"
				>
					<span class="r-name">
						{#if selectMode || selectedItems.has(`file:${file.id}`)}
							<button
								class="sel-check inline {selectedItems.has(`file:${file.id}`) ? 'on' : ''}"
								aria-label="Select"
								onclick={(e) => { e.stopPropagation(); toggleKey(`file:${file.id}`); }}
							>
								<Icon icon={selectedItems.has(`file:${file.id}`) ? 'ri:checkbox-fill' : 'ri:checkbox-blank-line'} width="18" />
							</button>
						{/if}
						<Icon
							icon={file.type === 'image' ? 'ri:image-2-fill' : file.type === 'video' ? 'ri:film-fill' : file.type === 'audio' ? 'ri:music-fill' : file.type === 'doc' ? 'ri:file-text-fill' : 'ri:file-fill'}
							width="20"
							class="r-icon"
						/>
						<span class="nm" title={file.name}>{file.name}</span>
						{#if file.encrypted}<Icon icon="ri:lock-fill" width="13" class="r-lock" />{/if}
						{#if file.starred}<Icon icon="ri:star-fill" width="14" class="r-star" />{/if}
					</span>
					<span class="r-size">{formatSize(file.size)}</span>
					<span class="r-date">{formatTime(file.created_on)}</span>
					<button
						class="r-menu"
						aria-label="Actions"
						onclick={(e) => { e.stopPropagation(); openItemContext(e, file, 'file'); }}
					>
						<Icon icon="ri:more-2-fill" width="18" />
					</button>
				</div>
			{/each}
		{/if}

		{#if hasMore && !isLoadingView}
			<div class="load-more-wrap">
				<button class="load-more" onclick={() => (visibleCount += PAGE_SIZE)}>
					Load more ({totalCount - visibleCount} remaining)
				</button>
			</div>
		{/if}
	</div>
</div>

{#if marquee.active}
	<div
		class="marquee-rect"
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
		placeholder="Enter Password"
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
				<button class="close-btn" onclick={() => (showUploadModal = false)} aria-label="Close">
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
	.dashboard-container {
		width: 100%;
		display: flex;
		flex-direction: column;
		gap: var(--space-6);
		height: 100%;
	}

	.dashboard-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		gap: var(--space-4);
		flex-wrap: wrap;

		.breadcrumbs-container {
			display: flex;
			align-items: center;
			gap: var(--space-4);
			min-width: 0;
		}

		.back-btn {
			background: var(--tint-soft);
			border: 1px solid var(--border-default);
			color: var(--text-primary);
			width: 36px;
			height: 36px;
			border-radius: var(--radius-sm);
			display: flex;
			align-items: center;
			justify-content: center;
			cursor: pointer;
			flex-shrink: 0;
			transition: background var(--dur) var(--ease), border-color var(--dur) var(--ease);

			&:hover {
				background: var(--tint-softer);
				color: var(--text-primary);
				border-color: var(--border-strong);
			}
		}

		.breadcrumbs {
			display: flex;
			align-items: center;
			gap: var(--space-2);
			font-size: var(--fs-lg);
			font-weight: var(--fw-medium);
			color: var(--text-muted);
			flex-wrap: wrap;

			.divider {
				opacity: 0.4;
				font-size: var(--fs-sm);
			}

			.crumb {
				background: none;
				border: none;
				color: inherit;
				font: inherit;
				cursor: pointer;
				padding: var(--space-1) var(--space-2);
				border-radius: var(--radius-sm);
				transition: background var(--dur) var(--ease), color var(--dur) var(--ease);

				&:hover {
					background: var(--tint-soft);
					color: var(--text-primary);
				}

				&.active {
					color: var(--text-primary);
					font-weight: var(--fw-semibold);
					cursor: default;
					&:hover {
						background: none;
					}
				}
			}
		}

		.header-actions {
			display: flex;
			gap: var(--space-3);

			.action-btn {
				display: flex;
				align-items: center;
				gap: var(--space-2);
				padding: 0.6rem 1rem;
				border-radius: var(--radius-pill);
				font-weight: var(--fw-semibold);
				font-size: var(--fs-sm);
				cursor: pointer;
				border: 1px solid transparent;
				background: var(--accent-gradient);
				color: #fff;
				box-shadow: 0 6px 18px -6px var(--primary-glow);
				transition: filter var(--dur) var(--ease), background var(--dur) var(--ease),
					border-color var(--dur) var(--ease), transform var(--dur) var(--ease);

				&:hover {
					filter: brightness(1.06);
					transform: translateY(-1px);
				}

				&.outline {
					background: var(--tint-soft);
					border-color: var(--border-default);
					color: var(--text-primary);
					box-shadow: none;
					&:hover {
						filter: none;
						background: var(--tint-softer);
						border-color: var(--border-strong);
					}
				}
			}
		}
	}

	.resource-area {
		position: relative;
		min-height: 200px;
		padding-bottom: var(--space-8);

		&.grid {
			display: grid;
			grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
			gap: var(--space-5);
		}
		&.list {
			display: flex;
			flex-direction: column;
			gap: 2px;
		}

		.drag-overlay {
			position: absolute;
			inset: 0;
			background: rgba(0, 0, 0, 0.78);
			backdrop-filter: blur(8px);
			z-index: 50;
			display: flex;
			flex-direction: column;
			align-items: center;
			justify-content: center;
			gap: var(--space-4);
			border-radius: var(--radius-md);
			border: 2px dashed var(--primary);
			color: var(--text-primary);
			font-weight: var(--fw-semibold);
			pointer-events: none;
		}

		.empty-state {
			grid-column: 1 / -1;
			display: flex;
			flex-direction: column;
			align-items: center;
			justify-content: center;
			padding: var(--space-10) 0;
			gap: var(--space-4);
			color: var(--text-secondary);

			p {
				font-size: var(--fs-body);
				font-weight: var(--fw-medium);
			}
			.text-btn {
				background: none;
				border: none;
				color: var(--primary);
				cursor: pointer;
				font-weight: var(--fw-semibold);
				margin-top: var(--space-2);
				font-size: var(--fs-sm);
				transition: color var(--dur) var(--ease);

				&:hover {
					color: var(--primary-hover);
					text-decoration: underline;
				}
			}
		}
	}

	/* ---- Toolbar ---- */
	.files-toolbar {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		flex-wrap: wrap;
	}
	.search-box {
		flex: 1;
		min-width: 200px;
		display: flex;
		align-items: center;
		gap: var(--space-2);
		background: var(--bg-input);
		border: 1px solid var(--border-default);
		border-radius: var(--radius-md);
		padding: 0 var(--space-3);
		color: var(--text-muted);
		transition: border-color var(--dur) var(--ease), box-shadow var(--dur) var(--ease);

		&:focus-within {
			border-color: var(--primary);
			box-shadow: 0 0 0 3px var(--primary-glow);
		}
		input {
			flex: 1;
			min-width: 0;
			background: transparent;
			border: none;
			outline: none;
			color: var(--text-primary);
			font-family: inherit;
			font-size: var(--fs-sm);
			padding: 0.6rem 0;
		}
		.clear-search {
			background: none;
			border: none;
			color: var(--text-muted);
			cursor: pointer;
			display: flex;
			padding: 2px;
			border-radius: var(--radius-sm);
			&:hover {
				color: var(--text-primary);
			}
		}
	}
	.toolbar-right {
		display: flex;
		align-items: center;
		gap: var(--space-2);
	}
	.sort-group {
		display: flex;
		background: var(--tint-soft);
		border: 1px solid var(--border-default);
		border-radius: var(--radius-md);
		padding: 2px;
		gap: 2px;

		.sort-btn {
			display: inline-flex;
			align-items: center;
			gap: 2px;
			background: transparent;
			border: none;
			color: var(--text-muted);
			font-family: inherit;
			font-size: var(--fs-sm);
			font-weight: var(--fw-medium);
			padding: var(--space-2) var(--space-3);
			border-radius: var(--radius-sm);
			cursor: pointer;
			transition: background var(--dur) var(--ease), color var(--dur) var(--ease);

			&:hover {
				color: var(--text-primary);
			}
			&.active {
				background: var(--bg-elevated);
				color: var(--text-primary);
				box-shadow: var(--shadow-card);
			}
		}
	}
	.tool-icon,
	.view-toggle button {
		background: var(--tint-soft);
		border: 1px solid var(--border-default);
		color: var(--text-muted);
		cursor: pointer;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: var(--space-2);
		border-radius: var(--radius-sm);
		transition: background var(--dur) var(--ease), color var(--dur) var(--ease);

		&:hover {
			color: var(--text-primary);
		}
		&.active {
			background: var(--primary);
			color: #fff;
			border-color: var(--primary);
		}
	}
	.tool-icon {
		border-radius: var(--radius-md);
	}
	.view-toggle {
		display: flex;
		border: 1px solid var(--border-default);
		border-radius: var(--radius-md);
		overflow: hidden;
		button {
			border: none;
			border-radius: 0;
		}
	}

	/* ---- Bulk action bar ---- */
	.bulk-bar {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-3);
		flex-wrap: wrap;
		background: var(--bg-elevated);
		border: 1px solid var(--border-active, var(--border-default));
		border-radius: var(--radius-md);
		padding: var(--space-2) var(--space-3);
		box-shadow: var(--shadow-card);

		.bulk-left {
			display: flex;
			align-items: center;
			gap: var(--space-3);
		}
		.bulk-x {
			background: transparent;
			border: none;
			color: var(--text-muted);
			cursor: pointer;
			display: flex;
			padding: 4px;
			border-radius: var(--radius-sm);
			&:hover {
				color: var(--text-primary);
				background: var(--tint-soft);
			}
		}
		.bulk-count {
			font-weight: var(--fw-semibold);
			color: var(--text-primary);
			font-size: var(--fs-sm);
		}
		.bulk-link {
			background: none;
			border: none;
			color: var(--primary);
			cursor: pointer;
			font-size: var(--fs-sm);
			font-weight: var(--fw-medium);
			&:disabled {
				color: var(--text-dim);
				cursor: default;
			}
		}
		.bulk-actions {
			display: flex;
			gap: var(--space-1);
			flex-wrap: wrap;
		}
		.bulk-btn {
			display: inline-flex;
			align-items: center;
			gap: var(--space-1);
			background: transparent;
			border: 1px solid transparent;
			color: var(--text-secondary);
			font-family: inherit;
			font-size: var(--fs-sm);
			font-weight: var(--fw-medium);
			padding: var(--space-2) var(--space-3);
			border-radius: var(--radius-sm);
			cursor: pointer;
			transition: background var(--dur) var(--ease), color var(--dur) var(--ease);
			&:hover {
				background: var(--tint-soft);
				color: var(--text-primary);
			}
			&.danger {
				color: var(--danger);
				&:hover {
					background: rgba(255, 70, 85, 0.1);
				}
			}
		}
	}

	/* ---- Grid cell wrapper ---- */
	.cell {
		position: relative;
		border-radius: var(--radius-md);
		transition: box-shadow var(--dur) var(--ease);

		/* Inset ring on the card itself so the 2px selection border never
		   overflows the grid edge (which clipped the leftmost column). */
		&.is-selected :global(.file-card),
		&.is-selected :global(.folder-card) {
			box-shadow: inset 0 0 0 2px var(--primary);
			border-color: var(--primary);
		}
	}
	.sel-check {
		position: absolute;
		top: var(--space-2);
		left: var(--space-2);
		z-index: 5;
		background: var(--bg-elevated);
		border: none;
		border-radius: var(--radius-sm);
		color: var(--text-muted);
		cursor: pointer;
		display: flex;
		padding: 1px;
		box-shadow: var(--shadow-card);
		&.on {
			color: var(--primary);
		}
		&.inline {
			position: static;
			box-shadow: none;
			background: transparent;
			padding: 0;
			margin-right: 2px;
		}
	}

	/* ---- Skeletons ---- */
	.skeleton {
		border-radius: var(--radius-md);
		background: linear-gradient(
			100deg,
			var(--tint-soft) 30%,
			var(--tint-softer) 50%,
			var(--tint-soft) 70%
		);
		background-size: 200% 100%;
		animation: shimmer 1.3s infinite;
		&.grid {
			min-height: 160px;
		}
		&.list {
			height: 52px;
		}
	}
	@keyframes shimmer {
		from {
			background-position: 200% 0;
		}
		to {
			background-position: -200% 0;
		}
	}

	/* ---- List view ---- */
	.list-head,
	.row {
		display: grid;
		grid-template-columns: 1fr 110px 150px 40px;
		align-items: center;
		gap: var(--space-3);
		padding: 0 var(--space-3);
	}
	.list-head {
		height: 36px;
		font-size: var(--fs-xs);
		text-transform: uppercase;
		letter-spacing: 0.04em;
		color: var(--text-dim);
		border-bottom: 1px solid var(--hairline);
		position: sticky;
		top: 0;
		background: var(--bg-base, transparent);
		z-index: 2;
		.lh-size,
		.lh-date {
			text-align: left;
		}
	}
	.row {
		height: 52px;
		border-radius: var(--radius-sm);
		cursor: pointer;
		color: var(--text-secondary);
		transition: background var(--dur) var(--ease), box-shadow var(--dur) var(--ease);
		user-select: none;

		&:hover {
			background: var(--bg-card-hover);
			.r-menu {
				opacity: 1;
			}
		}
		&.is-selected {
			background: var(--tint-soft);
			box-shadow: inset 2px 0 0 var(--primary);
		}

		.r-name {
			display: flex;
			align-items: center;
			gap: var(--space-2);
			min-width: 0;
			:global(.r-icon) {
				color: var(--text-secondary);
				flex: none;
			}
			:global(.r-icon.folder) {
				color: var(--primary);
			}
			.nm {
				overflow: hidden;
				text-overflow: ellipsis;
				white-space: nowrap;
				color: var(--text-primary);
				font-weight: var(--fw-medium);
				font-size: var(--fs-sm);
			}
			:global(.r-star) {
				color: var(--warning);
				flex: none;
			}
			:global(.r-lock) {
				color: var(--warning);
				flex: none;
			}
		}
		.r-size,
		.r-date {
			font-size: var(--fs-xs);
			color: var(--text-muted);
			font-family: var(--font-mono);
		}
		.r-menu {
			background: transparent;
			border: none;
			color: var(--text-muted);
			cursor: pointer;
			display: flex;
			justify-content: center;
			padding: var(--space-1);
			border-radius: var(--radius-sm);
			opacity: 0;
			transition: opacity var(--dur) var(--ease), color var(--dur) var(--ease),
				background var(--dur) var(--ease);
			&:hover {
				color: var(--text-primary);
				background: var(--tint-soft);
			}
		}
	}

	/* ---- Load more ---- */
	.load-more-wrap {
		grid-column: 1 / -1;
		display: flex;
		justify-content: center;
		padding: var(--space-5) 0 var(--space-2);
	}
	.load-more {
		background: var(--tint-soft);
		border: 1px solid var(--border-default);
		color: var(--text-secondary);
		font-family: inherit;
		font-weight: var(--fw-medium);
		font-size: var(--fs-sm);
		padding: var(--space-2) var(--space-5);
		border-radius: 999px;
		cursor: pointer;
		&:hover {
			color: var(--text-primary);
			background: var(--tint-softer);
		}
	}

	/* ---- Marquee selection rectangle ---- */
	.marquee-rect {
		position: fixed;
		z-index: 40;
		background: var(--primary-glow, rgba(99, 102, 241, 0.15));
		border: 1px solid var(--primary);
		border-radius: 2px;
		pointer-events: none;
	}

	@media (max-width: 640px) {
		.list-head {
			grid-template-columns: 1fr 70px 40px;
			.lh-date {
				display: none;
			}
		}
		.row {
			grid-template-columns: 1fr 70px 40px;
			.r-date {
				display: none;
			}
		}
	}

	/* Bulk-download per-file password modal */
	.bulkpw-intro {
		margin: 0;
		font-size: var(--fs-sm);
		color: var(--text-muted);
	}
	.bulkpw-list {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
		max-height: 320px;
		overflow-y: auto;
	}
	.bulkpw-row {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		flex-wrap: wrap;
	}
	.bulkpw-file {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		min-width: 0;
		flex: 1 1 160px;
		color: var(--text-secondary);
		font-size: var(--fs-sm);
		:global(svg) {
			color: var(--warning);
			flex-shrink: 0;
		}
		span {
			overflow: hidden;
			text-overflow: ellipsis;
			white-space: nowrap;
			color: var(--text-primary);
		}
	}
	.bulkpw-row input {
		flex: 1 1 160px;
		min-width: 0;
		background: var(--bg-input);
		border: 1px solid var(--border-default);
		border-radius: var(--radius-sm);
		padding: 0.6rem 0.8rem;
		color: var(--text-primary);
		font-family: var(--font-mono);
		font-size: var(--fs-sm);
		outline: none;
		transition: border-color var(--dur) var(--ease), box-shadow var(--dur) var(--ease);
		&:focus {
			border-color: var(--primary);
			box-shadow: 0 0 0 3px var(--primary-glow);
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
		backdrop-filter: blur(8px);
		z-index: 1000;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: var(--gutter);
	}

	.modal-content {
		background: var(--bg-elevated);
		border: 1px solid var(--border-default);
		border-radius: var(--radius-lg);
		box-shadow: var(--shadow-lg);
		width: 100%;
		max-width: 400px;
		display: flex;
		flex-direction: column;
		max-height: 90vh;

		&.upload-modal {
			max-width: 540px;
		}

		.modal-header {
			padding: var(--space-5) var(--space-5) var(--space-4);
			display: flex;
			align-items: center;
			justify-content: space-between;

			.modal-title {
				display: flex;
				align-items: center;
				gap: var(--space-2);
				font-weight: var(--fw-semibold);
				font-size: var(--fs-lg);
				color: var(--text-primary);
			}

			.close-btn {
				background: transparent;
				border: none;
				color: var(--text-muted);
				cursor: pointer;
				display: flex;
				padding: 4px;
				margin: -4px;
				border-radius: var(--radius-sm);
				transition: color var(--dur) var(--ease), background var(--dur) var(--ease);
				&:hover {
					color: var(--text-primary);
					background: var(--tint-soft);
				}
			}
		}

		.modal-body {
			flex: 1;
			overflow-y: auto;
			padding: 0 var(--space-5) var(--space-5);
			display: flex;
			flex-direction: column;
			gap: var(--space-4);
		}

		.modal-footer {
			padding: var(--space-4) var(--space-5);
			border-top: 1px solid var(--hairline);
			display: flex;
			justify-content: flex-end;
			gap: var(--space-3);
		}
	}

	.upload-area {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		min-height: 168px;
		border: 1.5px dashed var(--border-strong);
		border-radius: var(--radius-md);
		cursor: pointer;
		transition: border-color var(--dur) var(--ease), background var(--dur) var(--ease);
		color: var(--text-secondary);
		text-align: center;
		padding: var(--space-6) var(--space-5);

		.upload-area-icon {
			display: grid;
			place-items: center;
			width: 52px;
			height: 52px;
			border-radius: 50%;
			background: var(--tint-soft);
			color: var(--primary);
			margin-bottom: var(--space-3);
			transition: transform var(--dur) var(--ease);
		}

		&:hover,
		&.active {
			border-color: var(--primary);
			background: var(--tint-soft);
			.upload-area-icon {
				transform: translateY(-2px);
			}
		}

		p {
			margin: 0;
			font-weight: var(--fw-medium);
			color: var(--text-primary);
		}
		.link {
			color: var(--primary);
			font-weight: var(--fw-semibold);
		}
		.sub-text {
			font-size: var(--fs-xs);
			color: var(--text-muted);
			margin-top: var(--space-1);
		}
	}

	.encrypt-toggle {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-3);
		width: 100%;
		background: var(--tint-soft);
		border: 1px solid var(--hairline);
		border-radius: var(--radius-md);
		padding: var(--space-3) var(--space-4);
		cursor: pointer;
		text-align: left;
		transition: border-color var(--dur) var(--ease);

		&.on {
			border-color: var(--primary);
		}

		.encrypt-text {
			display: flex;
			align-items: center;
			gap: var(--space-3);
			color: var(--text-muted);
			min-width: 0;

			> span {
				display: flex;
				flex-direction: column;
				min-width: 0;
			}
			.t1 {
				color: var(--text-primary);
				font-weight: var(--fw-medium);
				font-size: var(--fs-sm);
			}
			.t2 {
				color: var(--text-muted);
				font-size: var(--fs-xs);
			}
		}

		.switch {
			flex: none;
			width: 38px;
			height: 22px;
			border-radius: 999px;
			background: var(--border-strong);
			position: relative;
			transition: background var(--dur) var(--ease);

			.knob {
				position: absolute;
				top: 2px;
				left: 2px;
				width: 18px;
				height: 18px;
				border-radius: 50%;
				background: #fff;
				transition: transform var(--dur) var(--ease);
			}
		}
		&.on .switch {
			background: var(--primary);
			.knob {
				transform: translateX(16px);
			}
		}
	}

	.password-input {
		display: flex;
		gap: var(--space-2);

		.input-field {
			flex: 1;
			min-width: 0;
			background: var(--bg-input);
			border: 1px solid var(--border-default);
			padding: 0.75rem 0.95rem;
			border-radius: var(--radius-sm);
			color: var(--text-primary);
			font-family: var(--font-mono);
			outline: none;
			transition: border-color var(--dur) var(--ease), box-shadow var(--dur) var(--ease);
			&:focus {
				border-color: var(--primary);
				box-shadow: 0 0 0 3px var(--primary-glow);
			}
		}
		.pw-icon-btn {
			flex: none;
			display: grid;
			place-items: center;
			width: 40px;
			background: var(--tint-soft);
			border: 1px solid var(--border-default);
			border-radius: var(--radius-sm);
			color: var(--text-muted);
			cursor: pointer;
			transition: color var(--dur) var(--ease), background var(--dur) var(--ease);
			&:hover:not(:disabled) {
				color: var(--text-primary);
				background: var(--tint-softer);
			}
			&:disabled {
				opacity: 0.45;
				cursor: not-allowed;
			}
		}
		.generate-btn {
			padding: 0 var(--space-3);
			background: var(--tint-softer);
			border: 1px solid var(--border-default);
			border-radius: var(--radius-sm);
			color: var(--text-primary);
			cursor: pointer;
			font-size: var(--fs-sm);
			font-weight: var(--fw-medium);
			flex: none;
			&:hover {
				background: var(--bg-card-hover);
			}
		}
	}

	.selected-files {
		.selected-files-head {
			display: flex;
			align-items: center;
			justify-content: space-between;
			margin-bottom: var(--space-3);
			h3 {
				font-size: var(--fs-sm);
				color: var(--text-secondary);
				margin: 0;
			}
			.clear-btn {
				background: none;
				border: none;
				color: var(--text-muted);
				font-size: var(--fs-xs);
				cursor: pointer;
				&:hover {
					color: var(--primary);
				}
			}
		}
		.file-list-scroll {
			max-height: 168px;
			overflow-y: auto;
			display: flex;
			flex-direction: column;
			gap: var(--space-2);
			&::-webkit-scrollbar {
				width: 4px;
			}
			&::-webkit-scrollbar-thumb {
				background: var(--border-strong);
				border-radius: 999px;
			}
		}
		.selected-file-row {
			display: flex;
			align-items: center;
			gap: var(--space-3);
			background: var(--bg-input);
			padding: var(--space-2) var(--space-3);
			border-radius: var(--radius-sm);
			font-size: var(--fs-sm);
			color: var(--text-muted);
			.name {
				flex: 1;
				overflow: hidden;
				text-overflow: ellipsis;
				white-space: nowrap;
				color: var(--text-primary);
			}
			.size {
				color: var(--text-muted);
				font-size: var(--fs-xs);
				font-family: var(--font-mono);
			}
			.remove-btn {
				background: transparent;
				border: none;
				color: var(--text-muted);
				cursor: pointer;
				display: flex;
				&:hover {
					color: var(--danger);
				}
			}
		}
	}

	.count-pill {
		display: inline-grid;
		place-items: center;
		min-width: 18px;
		height: 18px;
		padding: 0 5px;
		margin-left: 6px;
		border-radius: 999px;
		background: rgba(255, 255, 255, 0.22);
		font-size: var(--fs-xs);
		font-weight: var(--fw-semibold);
	}

	/* Bottom-sheet on small screens */
	@media (max-width: 600px) {
		.modal-backdrop {
			align-items: flex-end;
			padding: 0;
		}
		.modal-content.upload-modal {
			max-width: 100%;
			max-height: 92vh;
			border-radius: var(--radius-lg) var(--radius-lg) 0 0;
			border-bottom: none;
		}
		.modal-content.upload-modal .modal-header,
		.modal-content.upload-modal .modal-body {
			padding-left: var(--space-4);
			padding-right: var(--space-4);
		}
		.modal-content.upload-modal .modal-footer {
			padding: var(--space-4);
			.btn {
				flex: 1;
			}
		}
	}

	.upload-progress-container {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		flex: 1;
		gap: var(--space-5);

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
				stroke: var(--tint-softer);
				stroke-width: 2.5;
			}
			.circle {
				fill: none;
				stroke: var(--primary);
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
				font-size: var(--fs-h3);
				font-weight: var(--fw-bold);
				font-family: var(--font-mono);
				color: var(--text-primary);
			}
		}

		.upload-details {
			text-align: center;
			h3 {
				font-size: var(--fs-lg);
				color: var(--text-primary);
				margin-bottom: var(--space-2);
			}
			.current-file {
				color: var(--text-secondary);
				font-size: var(--fs-sm);
				margin-bottom: var(--space-1);
				max-width: 300px;
				overflow: hidden;
				text-overflow: ellipsis;
				white-space: nowrap;
			}
			.stats-row {
				font-size: var(--fs-sm);
				color: var(--text-muted);
				font-family: var(--font-mono);
				margin-top: var(--space-2);
			}
		}
	}

	.selected-item {
		background-color: var(--tint-soft);
		border-radius: var(--radius-sm);
		border: 1px solid var(--primary);
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
		border-radius: var(--radius-md);
		border: 2px dashed var(--primary);
		background-color: var(--tint-soft);
		pointer-events: none;
	}

	.drag-target-active-crumb {
		outline: 2px dashed var(--primary);
		background-color: var(--tint-soft);
		border-radius: var(--radius-sm);
		color: var(--primary) !important;
	}

	@media (max-width: 600px) {
		.dashboard-header {
			align-items: flex-start;
		}
		.header-actions {
			width: 100%;
		}
		.header-actions .action-btn {
			flex: 1;
			justify-content: center;
		}
		.resource-area.grid {
			grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
			gap: var(--space-3);
		}
		.files-toolbar {
			gap: var(--space-2);
		}
		.search-box {
			order: -1;
			width: 100%;
			flex-basis: 100%;
		}
		.toolbar-right {
			width: 100%;
			justify-content: space-between;
		}
		.sort-group {
			flex: 1;
		}
		.bulk-bar .bulk-btn span {
			display: none;
		}
	}
</style>
