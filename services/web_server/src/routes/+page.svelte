<script>
	import Icon from '@iconify/svelte';
	import SiloCatLogo from '$lib/assets/silo-cat.png';
	import ClickswaveLogo from '$lib/assets/clickswave_transparent.webp';
	import { fade } from 'svelte/transition';
	import Navbar from '$lib/components/Navbar.svelte';
	import Footer from '$lib/components/Footer.svelte';

	import { createMutation } from '@tanstack/svelte-query';
	import { toast } from 'svelte-sonner';
	import axios from 'axios';

	import { shadowKey } from '$lib/stores/shadow.js';

	let { data } = $props();

	// Use store or authenticated user key
	let api_key = $derived(data?.user?.api_key || $shadowKey);

	let files = $state([]);
	let isDragging = $state(false);
	let isUploading = $state(false);
	let uploadSuccessUrl = $state(null);
	let showSuccessModal = $state(false);
	let uploadType = $state('file'); // 'file', 'files', 'folder'

	const MAX_SIZE_ANON = 20 * 1024 * 1024 * 1024; // 20GB
	const MAX_SIZE_AUTH = 50 * 1024 * 1024 * 1024; // 50GB

	function checkSizeLimit(newFiles) {
		// Strict check for authenticated user: must have email and not be anonymous
		const isAuth = data?.user && data.user.email && data.user.account_type !== 'anonymous';
		const limit = isAuth ? MAX_SIZE_AUTH : MAX_SIZE_ANON;
		const currentTotal = files.reduce((acc, f) => acc + (f.file ? f.file.size : f.size), 0);
		const newTotal = newFiles.reduce((acc, f) => acc + (f.file ? f.file.size : f.size), 0);

		if (currentTotal + newTotal > limit) {
			const limitGb = limit / (1024 * 1024 * 1024);
			toast.error(
				`Total upload size exceeds the ${limitGb}GB limit for ${isAuth ? 'authenticated' : 'anonymous'} users.`
			);
			return false;
		}
		return true;
	}

	// Upload Progress State
	let uploadStats = $state({
		totalBytes: 0,
		uploadedBytes: 0,
		speed: 0, // bytes per second
		eta: 0, // seconds
		totalProgress: 0, // 0-100
		fileProgress: 0, // 0-100 current file
		chunkProgress: 0, // 0-100 current chunk
		startTime: 0,
		currentFileName: ''
	});

	function copySuccessLink() {
		if (uploadSuccessUrl) {
			navigator.clipboard.writeText(uploadSuccessUrl);
			toast.success('Download link copied!');
		}
	}

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
						// Fallback
						const file = item.getAsFile();
						if (file) newFiles.push({ file, path: '' });
					}
				}
			}
			if (checkSizeLimit(newFiles)) {
				files = [...files, ...newFiles];
			}
		} else if (e.dataTransfer.files) {
			const newFiles = Array.from(e.dataTransfer.files).map((f) => ({ file: f, path: '' }));
			if (checkSizeLimit(newFiles)) {
				files = [...files, ...newFiles];
			}
		}
	}

	function handleFileSelect(e) {
		if (e.target.files) {
			const selected = Array.from(e.target.files);
			const newFiles = selected.map((f) => {
				const relativePath = f.webkitRelativePath
					? f.webkitRelativePath.split('/').slice(0, -1).join('/')
					: '';
				return { file: f, path: relativePath };
			});
			if (checkSizeLimit(newFiles)) {
				files = [...files, ...newFiles];
			}
		}
	}

	function removeFile(index) {
		files = files.filter((_, i) => i !== index);
	}

	let encryptionEnabled = $state(false);
	let password = $state('');

	function generatePassword() {
		const chars = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%^&*';
		let pass = '';
		for (let i = 0; i < 16; i++) {
			pass += chars.charAt(Math.floor(Math.random() * chars.length));
		}
		return pass;
	}

	function toggleEncryption() {
		encryptionEnabled = !encryptionEnabled;
		if (encryptionEnabled && !password) {
			password = generatePassword();
		}
	}

	function copyPassword() {
		if (password) {
			navigator.clipboard.writeText(password);
			toast.success('Password copied to clipboard');
		}
	}

	// --- Upload Logic ---

	// Worker Integration
	import CryptoWorker from '$lib/workers/crypto.worker.js?worker';
	import sodium from 'libsodium-wrappers-sumo'; // Still need for ready check if mutation uses it, but try to minimize

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

	const CHUNK_SIZE = 100 * 1024 * 1024; // 100MB

	async function getFileChecksum(file) {
		return callWorker('hashFile', { file, chunkSize: CHUNK_SIZE });
	}

	async function deriveKeyFromPasswordWorker(password, salt) {
		return callWorker('deriveKey', { password, salt });
	}

	async function encryptChunkWorker(chunkBuffer, key, nonce) {
		return callWorker('encryptChunk', { chunk: chunkBuffer, key, nonce }, [chunkBuffer.buffer]);
	}

	// Helpers for quick generation on main thread
	function generateSalt() {
		return sodium.randombytes_buf(sodium.crypto_pwhash_SALTBYTES);
	}
	function generateNonce() {
		return sodium.randombytes_buf(sodium.crypto_secretbox_NONCEBYTES);
	}

	// Upload Mutation now focuses on UPLOADING content for an already created file
	// params: { file, fileId, serverChunks, encryptionParams, onProgress }
	const uploadMutation = createMutation(() => ({
		mutationFn: async (params) => {
			const { file, fileId, serverChunks, encryptionParams, onProgress } = params;

			// Re-use encryption params if provided (calculated in Phase 1)
			let key = encryptionParams?.key;
			// salt is part of encryptionParams but not needed directly for chunk upload unless we re-derive
			let chunksMeta = encryptionParams?.chunksMeta;

			// Verify file checksum if not done? (Done in Phase 1)

			const totalChunks = serverChunks.length;
			let fileUploadedBytes = 0;

			for (let i = 0; i < totalChunks; i++) {
				const chunkMeta = chunksMeta[i];
				const serverChunk = serverChunks[i];

				const chunkBlob = file.slice(chunkMeta.start, chunkMeta.end);
				const chunkBuffer = new Uint8Array(await chunkBlob.arrayBuffer());

				let dataToUpload = chunkBuffer;
				if (encryptionEnabled) {
					// We need the key here.
					if (!key) throw new Error('Encryption key missing for upload');
					dataToUpload = await encryptChunkWorker(chunkBuffer, key, chunkMeta._rawNonce);
				}

				// Upload to R2
				const MAX_RETRIES = 3;
				let lastError;

				for (let attempt = 1; attempt <= MAX_RETRIES; attempt++) {
					try {
						await axios.put(serverChunk.presigned_url, dataToUpload, {
							headers: { 'Content-Type': 'application/octet-stream' },
							onUploadProgress: (progressEvent) => {
								const chunkRatio = progressEvent.loaded / progressEvent.total;
								const chunkProgress = chunkRatio * 100;
								const currentChunkBytes = chunkMeta.size * chunkRatio;

								if (onProgress) {
									onProgress(chunkProgress, fileUploadedBytes + currentChunkBytes);
								}
							}
						});
						lastError = null;
						break;
					} catch (e) {
						lastError = e;
						if (attempt < MAX_RETRIES) await new Promise((r) => setTimeout(r, 1000 * attempt));
					}
				}
				if (lastError) throw lastError;

				fileUploadedBytes += chunkMeta.size;

				// Mark Complete
				await axios.post(
					'/api/v1/shadow/file/mark-chunk-complete',
					{ chunk_id: serverChunk.id },
					{ headers: { 'X-Api-Key': api_key } }
				);
			}

			// Return something if needed, or just success
			return { success: true };
		},
		onError: (error) => {
			console.error('Upload failed', error);
			toast.error(`Failed to upload file: ${error.message}`);
		}
	}));

	// Folder Cache
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

			if (folderCache.has(currentPath)) {
				currentParentId = folderCache.get(currentPath);
				continue;
			}

			try {
				// Shadow folder creation requires X-Api-Key if we want to link it to the user/session
				// Assuming /api/v1/shadow/folder works similarly to sanctum but needs headers
				const res = await axios.post(
					'/api/v1/shadow/folder',
					{
						name: part,
						parent_id: currentParentId,
						uploaded_as_files: false
					},
					{
						headers: { 'X-Api-Key': api_key }
					}
				);

				// Inspect structure
				if (res.data?.data?.folder?.id) {
					currentParentId = res.data.data.folder.id;
					folderCache.set(currentPath, currentParentId);
				} else if (res.data?.data?.id) {
					currentParentId = res.data.data.id;
					folderCache.set(currentPath, currentParentId);
				}
			} catch (e) {
				console.warn(`Could not create folder ${part}, trying to find existing...`, e);
				// Fallback implemented?
				// Shadow API might not have a "list" endpoint that's easily searchable here?
				// The legacy code didn't have robust retry. Let's try basic create and fail if not.
				// Or maybe we can skip this part if it fails and upload to root?
				// For now, let's propagate error as it's better than silent failure.
				throw e;
			}
		}
		return currentParentId;
	}

	async function startUpload() {
		if (files.length === 0) return;
		isUploading = true;
		folderCache.clear();

		uploadStats.totalBytes = files.reduce((acc, f) => acc + (f.file ? f.file.size : f.size), 0);
		uploadStats.uploadedBytes = 0;
		uploadStats.startTime = Date.now();
		uploadStats.totalProgress = 0;

		let globalUploadedBytesBase = 0;
		let successCount = 0;
		let topLevelFolderId = null;

		try {
			await sodium.ready; // Ensure sodium ready for batch ops

			// --- PHASE 0: Create Batch Folder if needed ---
			if (
				files.length > 1 &&
				files.every((f) => !f.path && !(f.file && f.file.webkitRelativePath))
			) {
				try {
					const timestamp = new Date()
						.toLocaleString('en-US', {
							month: 'short',
							day: 'numeric',
							hour: 'numeric',
							minute: 'numeric',
							hour12: true
						})
						.replace(/,/g, '');
					const batchFolderName = `Upload ${timestamp}`;
					const res = await axios.post(
						'/api/v1/shadow/folder',
						{ name: batchFolderName, parent_id: null, uploaded_as_files: false },
						{ headers: { 'X-Api-Key': api_key } }
					);
					if (res.data?.data?.folder?.id) topLevelFolderId = res.data.data.folder.id;
					else if (res.data?.data?.id) topLevelFolderId = res.data.data.id;

					if (topLevelFolderId) {
						uploadSuccessUrl = `${window.location.origin}/${topLevelFolderId}`;
						showSuccessModal = true;
					}
				} catch (e) {
					console.warn('Failed to create batch folder', e);
				}
			}

			// --- PHASE 1: Pre-Process & Create All Records ---
			// We calculate checksums and encryption meta here to create the file record.
			// This might delay "visible" start if many files, but ensures instant availability once "uploading" phase starts.
			const pendingUploads = [];

			for (const fileItem of files) {
				const file = fileItem.file || fileItem;
				const relativePath =
					fileItem.path ||
					(file.webkitRelativePath
						? file.webkitRelativePath.split('/').slice(0, -1).join('/')
						: null);

				let folderId = topLevelFolderId;
				if (relativePath) {
					folderId = await ensureFolderExists(relativePath);
					if (!topLevelFolderId) {
						const firstPart = relativePath.split('/')[0];
						if (folderCache.has(firstPart)) topLevelFolderId = folderCache.get(firstPart);
						// Show modal ASAP for folder structure
						if (topLevelFolderId && !uploadSuccessUrl) {
							uploadSuccessUrl = `${window.location.origin}/${topLevelFolderId}`;
							showSuccessModal = true;
						}
					}
				}

				// Heavy lifting: Checksum + Encryption Meta
				const fileChecksum = await getFileChecksum(file);
				let key = null;
				let salt = null;
				if (encryptionEnabled) {
					salt = generateSalt();
					key = await deriveKeyFromPasswordWorker(password, salt);
				}

				const totalChunks = Math.ceil(file.size / CHUNK_SIZE);
				const chunksMeta = [];
				for (let i = 0; i < totalChunks; i++) {
					const start = i * CHUNK_SIZE;
					const end = Math.min(start + CHUNK_SIZE, file.size);
					const size = end - start;
					let chunkNonce = null;
					let chunkSalt = null;
					if (encryptionEnabled) {
						chunkNonce = generateNonce();
						chunkSalt = salt;
					}
					chunksMeta.push({
						start,
						end,
						size,
						checksum: 'pending',
						nonce: chunkNonce ? btoa(String.fromCharCode(...chunkNonce)) : null,
						salt: chunkSalt ? btoa(String.fromCharCode(...chunkSalt)) : null,
						_rawNonce: chunkNonce
					});
				}

				// Create Record
				const payload = {
					storage_type: 'shadow',
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

				const res = await axios.post('/api/v1/shadow/file', payload, {
					headers: { 'X-Api-Key': api_key }
				});
				if (res.data.status !== 200 && res.data.status !== 201) throw new Error(res.data.message);

				// Instant Link for Single File (if not folder)
				if (!folderId && files.length === 1) {
					uploadSuccessUrl = `${window.location.origin}/${res.data.data.file.id}`;
					showSuccessModal = true;
				}

				pendingUploads.push({
					file,
					fileId: res.data.data.file.id,
					serverChunks: res.data.data.chunks,
					encryptionParams: { key, salt, chunksMeta }
				});
			}

			// --- PHASE 2: Upload Content (Sequential or Parallel) ---
			// Now that all files exist on server, listeners on /slug will see them.
			for (const task of pendingUploads) {
				uploadStats.currentFileName = task.file.name;
				uploadStats.fileProgress = 0;
				uploadStats.chunkProgress = 0;

				await uploadMutation.mutateAsync({
					file: task.file,
					fileId: task.fileId,
					serverChunks: task.serverChunks,
					encryptionParams: task.encryptionParams,
					onProgress: (chunkPct, fileBytes) => {
						uploadStats.chunkProgress = chunkPct;
						uploadStats.fileProgress = (fileBytes / task.file.size) * 100;
						const currentTotal = globalUploadedBytesBase + fileBytes;
						uploadStats.uploadedBytes = currentTotal;
						uploadStats.totalProgress = (currentTotal / uploadStats.totalBytes) * 100;

						const elapsed = (Date.now() - uploadStats.startTime) / 1000;
						if (elapsed > 0.5) {
							uploadStats.speed = currentTotal / elapsed;
							const remaining = uploadStats.totalBytes - currentTotal;
							uploadStats.eta = uploadStats.speed > 0 ? remaining / uploadStats.speed : 0;
						}
					}
				});
				globalUploadedBytesBase += task.file.size;
				successCount++;
			}

			if (successCount > 0) toast.success(`Upload complete! (${successCount} files)`);
		} catch (e) {
			console.error('Batch upload interrupted', e);
			const d = e?.response?.data;
			if (e?.response?.status === 403 && (d?.banned || /banned/i.test(d?.error || ''))) {
				toast.error(d?.error || 'You are banned from using SiloCat.');
			} else {
				toast.error('Upload failed or interrupted');
			}
		} finally {
			isUploading = false;
		}
	}
</script>

<svelte:head>
	<title>SiloCat - Cat powered file sharing and storage platform</title>
	<meta
		name="description"
		content="Kitty powered E2E encrypted anonymous file-sharing and cloud storage platform with parallel downloads. Zero-knowledge, built for the paranoid."
	/>
	<meta
		name="keywords"
		content="encrypted file sharing, secure storage, zero knowledge, end-to-end encryption, anonymous file share, parallel downloads"
	/>
	<meta property="og:title" content="SiloCat - Anonymous Encrypted File Sharing" />
	<meta
		property="og:description"
		content="Kitty powered E2E encrypted anonymous file-sharing and cloud storage platform with parallel downloads."
	/>
	<meta property="og:type" content="website" />
</svelte:head>

<div class="landing-page">
	<Navbar />

	<main class="hero">
		<div class="hero-content">
			<span class="eyebrow"><span class="paw">🐾</span> zero-knowledge file vault</span>
			<h1 class="hero-title">Big files.<br /><span class="text-gradient">Zero knowledge.</span></h1>
			<p class="subtitle">
				Kitty-powered, end-to-end encrypted, anonymous file sharing. Drop up to 20&nbsp;GB, get a link,
				and share it. We never see what's inside.
			</p>
			<div class="trust-row">
				<span class="chip"><Icon icon="ri:lock-2-line" /> End-to-end encrypted</span>
				<span class="chip"><Icon icon="ri:ghost-2-line" /> Anonymous</span>
				<span class="chip"><Icon icon="ri:flashlight-line" /> Parallel downloads</span>
				<span class="chip"><Icon icon="ri:hard-drive-2-line" /> 20GB free</span>
			</div>

			<div
				class="upload-zone {isDragging ? 'dragging' : ''}"
				ondragover={handleDragOver}
				ondragleave={handleDragLeave}
				ondrop={handleDrop}
				role="button"
				tabindex="0"
			>
				{#if files.length === 0}
					<div class="upload-placeholder">
						<div class="icon-circle">
							<Icon icon="ri:upload-cloud-2-line" width="48" />
						</div>
						<h3>Drag & drop files here</h3>
						<p>or</p>
						<label for="file-upload" class="browse-btn">Browse Files</label>
						<input type="file" id="file-upload" multiple onchange={handleFileSelect} hidden />
						<div class="limit-badge">
							<Icon icon="ri:hard-drive-2-line" /> Max file size: 20GB
						</div>
						<div class="folder-upload-hint">
							<label for="folder-upload" class="link-btn">or upload a folder</label>
							<input
								type="file"
								id="folder-upload"
								webkitdirectory
								directory
								multiple
								onchange={handleFileSelect}
								hidden
							/>
						</div>
					</div>
				{:else}
					<div class="file-list">
						{#each files as fileItem, i}
							{@const f = fileItem.file || fileItem}
							{@const path = fileItem.path || ''}
							<div class="file-item" transition:fade>
								<div class="file-icon">
									<Icon icon="ri:file-line" />
								</div>
								<div class="file-info">
									<span class="name">{f.name}</span>
									{#if path}
										<span class="path-hint">{path}/</span>
									{/if}
									<span class="size">{(f.size / (1024 * 1024)).toFixed(2)} MB</span>
								</div>
								<button class="remove-btn" onclick={() => removeFile(i)}>
									<Icon icon="ri:close-line" />
								</button>
							</div>
						{/each}
						<div class="encryption-section">
							<div class="toggle-row">
								<label class="toggle-switch">
									<input
										type="checkbox"
										checked={encryptionEnabled}
										onchange={toggleEncryption}
										disabled={isUploading}
									/>
									<span class="slider"></span>
								</label>
								<span class="toggle-label">Protect with Password</span>
							</div>

							{#if !encryptionEnabled}
								<div class="warning-box" transition:fade>
									<Icon icon="ri:alert-line" />
									<span>Warning: File will be uploaded without encryption.</span>
								</div>
							{/if}

							{#if encryptionEnabled}
								<div class="password-input-group" transition:fade>
									<input
										type="text"
										bind:value={password}
										placeholder="Enter password"
										class="password-field"
										disabled={isUploading}
									/>
									<button
										class="regen-btn"
										onclick={() => (password = generatePassword())}
										title="Generate new password"
										disabled={isUploading}
									>
										<Icon icon="ri:refresh-line" />
									</button>
									<button class="copy-btn" onclick={copyPassword} title="Copy password">
										<Icon icon="ri:file-copy-line" />
									</button>
								</div>
							{/if}
						</div>

						{#if isUploading}
							<div class="progress-section" transition:fade>
								<div class="stats-row">
									<div class="stat">
										<span class="label">Speed</span>
										<span class="value">{(uploadStats.speed / 1024 / 1024).toFixed(2)} MB/s</span>
									</div>
									<div class="stat">
										<span class="label">ETA</span>
										<span class="value">{Math.ceil(uploadStats.eta)}s</span>
									</div>
								</div>

								<!-- Total Progress -->
								<div class="progress-item">
									<div class="progress-header">
										<span class="label">Total Progress</span>
										<span class="percent">{Math.round(uploadStats.totalProgress)}%</span>
									</div>
									<div class="progress-track">
										<div
											class="progress-fill total"
											style="width: {uploadStats.totalProgress}%"
										></div>
									</div>
								</div>

								<!-- File Progress -->
								<div class="progress-item">
									<div class="progress-header">
										<span class="label trunc">{uploadStats.currentFileName}</span>
										<span class="percent">{Math.round(uploadStats.fileProgress)}%</span>
									</div>
									<div class="progress-track">
										<div
											class="progress-fill file"
											style="width: {uploadStats.fileProgress}%"
										></div>
									</div>
								</div>

								<!-- Chunk Progress (Only if meaningful/requested) -->
								<div class="progress-item compact">
									<div class="progress-track">
										<div
											class="progress-fill chunk"
											style="width: {uploadStats.chunkProgress}%"
										></div>
									</div>
									<span class="tiny-label">Chunk Upload</span>
								</div>
							</div>
						{/if}

						<button
							class="upload-action-btn"
							onclick={startUpload}
							disabled={isUploading || uploadMutation.isPending}
						>
							{#if isUploading || uploadMutation.isPending}
								<Icon icon="line-md:loading-loop" width="20" />
								Uploading...
							{:else}
								<Icon icon="ri:upload-cloud-2-line" width="20" />
								Start Upload
							{/if}
						</button>
						<button class="add-more-btn" onclick={() => (files = [])}>Clear All</button>
					</div>
				{/if}
			</div>

			<!-- Success Modal / Overlay -->
			{#if showSuccessModal}
				<div class="success-overlay" transition:fade>
					<div class="success-card">
						<div class="success-icon">
							{#if isUploading}
								<Icon icon="line-md:loading-loop" />
							{:else}
								<Icon icon="ri:checkbox-circle-fill" />
							{/if}
						</div>
						<h2>
							{#if isUploading}
								Uploading in Background...
							{:else}
								{files.length > 1 ? 'Files' : 'File'} Uploaded Successfully!
							{/if}
						</h2>
						{#if uploadSuccessUrl}
							<p>
								Your {files.length > 1 ? 'files are' : 'file is'}
								{isUploading
									? 'being uploaded. You can already share this link:'
									: 'ready to share. Anyone with this link can download it.'}
							</p>

							<div class="link-box">
								<input type="text" readonly value={uploadSuccessUrl} />
								<button onclick={copySuccessLink} class="copy-link-btn">
									<Icon icon="ri:file-copy-line" />
									Copy Link
								</button>
							</div>

							{#if isUploading}
								<div class="modal-progress" style="margin-top: 24px; width: 100%;">
									<!-- Current File Status -->
									<div style="margin-bottom: 12px;">
										<div
											class="label-row"
											style="display: flex; justify-content: space-between; font-size: 0.85rem; color: rgba(255,255,255,0.8); margin-bottom: 4px;"
										>
											<span
												style="white-space: nowrap; overflow: hidden; text-overflow: ellipsis; max-width: 70%;"
											>
												{uploadStats.currentFileName || 'Processing...'}
											</span>
											<span>{Math.round(uploadStats.fileProgress)}%</span>
										</div>
										<div
											class="progress-bar-bg"
											style="background: rgba(255, 255, 255, 0.1); border-radius: 4px; height: 6px; overflow: hidden;"
										>
											<div
												class="progress-bar-fill"
												style="width: {uploadStats.fileProgress}%; background: var(--secondary-color, #4facfe); height: 100%; transition: width 0.1s linear;"
											></div>
										</div>
									</div>

									<!-- Chunk Status -->
									<div style="margin-bottom: 12px;">
										<div
											class="label-row"
											style="display: flex; justify-content: space-between; font-size: 0.75rem; color: rgba(255,255,255,0.6); margin-bottom: 2px;"
										>
											<span>Current Chunk</span>
											<span>{Math.round(uploadStats.chunkProgress)}%</span>
										</div>
										<div
											class="progress-bar-bg"
											style="background: rgba(255, 255, 255, 0.05); border-radius: 2px; height: 4px; overflow: hidden;"
										>
											<div
												class="progress-bar-fill"
												style="width: {uploadStats.chunkProgress}%; background: rgba(255, 255, 255, 0.5); height: 100%; transition: width 0.05s linear;"
											></div>
										</div>
									</div>

									<!-- Total Progress -->
									<div>
										<div
											class="label-row"
											style="display: flex; justify-content: space-between; font-size: 0.85rem; color: rgba(255,255,255,0.8); margin-bottom: 4px;"
										>
											<span>Total Upload</span>
											<span>{Math.round(uploadStats.totalProgress)}%</span>
										</div>
										<div
											class="progress-bar-bg"
											style="background: rgba(255, 255, 255, 0.1); border-radius: 4px; height: 8px; overflow: hidden;"
										>
											<div
												class="progress-bar-fill"
												style="width: {uploadStats.totalProgress}%; background: #ff4655; height: 100%; transition: width 0.3s ease;"
											></div>
										</div>
									</div>

									<!-- Stats Row -->
									<div
										style="display: flex; justify-content: space-between; margin-top: 8px; font-size: 0.8rem; color: rgba(255,255,255,0.5);"
									>
										<span
											>{uploadStats.speed
												? (uploadStats.speed / 1024 / 1024).toFixed(1) + ' MB/s'
												: 'Starting...'}</span
										>
										<span>ETA: {uploadStats.eta ? Math.ceil(uploadStats.eta) + 's' : '--'}</span>
									</div>
								</div>
							{/if}
						{:else}
							<p>Your files have been uploaded successfully.</p>
						{/if}

						{#if encryptionEnabled && password}
							<div class="password-display-box" transition:fade>
								<div class="password-label">
									<Icon icon="ri:lock-password-line" />
									<span>Decryption Password</span>
								</div>
								<div class="password-value-row">
									<code class="password-code">{password}</code>
									<button
										class="copy-btn"
										onclick={() => {
											navigator.clipboard.writeText(password);
											toast.success('Password copied to clipboard!');
										}}
										title="Copy Password"
									>
										<Icon icon="ri:file-copy-line" />
									</button>
								</div>
								<p class="password-hint">
									Save this password securely. It is required to download the file.
								</p>
							</div>
						{/if}

						<div class="action-buttons">
							<button
								class="primary-btn"
								onclick={() => {
									uploadSuccessUrl = null;
									showSuccessModal = false;
									files = [];
									password = '';
									encryptionEnabled = false;
								}}
								disabled={isUploading}>Upload Another File</button
							>
						</div>
					</div>
				</div>
			{/if}
		</div>
	</main>

	<section class="section how">
		<div class="container">
			<span class="eyebrow">how it works</span>
			<h2 class="section-title">Three steps. No account needed.</h2>
			<div class="steps">
				<div class="step">
					<span class="step-n">01</span>
					<div class="step-ic"><Icon icon="ri:upload-cloud-2-line" width="26" /></div>
					<h3>Drop it</h3>
					<p>Drag in a file or a whole folder. Up to 20GB anonymously, 50GB with a free account.</p>
				</div>
				<div class="step">
					<span class="step-n">02</span>
					<div class="step-ic"><Icon icon="ri:lock-2-line" width="26" /></div>
					<h3>We encrypt it</h3>
					<p>Optional password protection encrypts every chunk in your browser. The key never leaves your device.</p>
				</div>
				<div class="step">
					<span class="step-n">03</span>
					<div class="step-ic"><Icon icon="ri:links-line" width="26" /></div>
					<h3>Share the link</h3>
					<p>Get a clean link the moment upload starts. Anyone with it pulls the file at full speed.</p>
				</div>
			</div>
		</div>
	</section>

	<section class="section features">
		<div class="container">
			<span class="eyebrow">why silocat</span>
			<h2 class="section-title">Built for the paranoid.</h2>
			<div class="feature-grid">
				<div class="feature"><div class="feature-ic"><Icon icon="ri:shield-keyhole-line" width="24" /></div><h3>Zero-knowledge</h3><p>Files are encrypted before they leave your browser. We store ciphertext, nothing else.</p></div>
				<div class="feature"><div class="feature-ic"><Icon icon="ri:ghost-2-line" width="24" /></div><h3>Truly anonymous</h3><p>No email or signup required to send. No tracking, no profiling, no ads.</p></div>
				<div class="feature"><div class="feature-ic"><Icon icon="ri:flashlight-line" width="24" /></div><h3>Parallel downloads</h3><p>Chunked storage saturates your connection instead of trickling one stream.</p></div>
				<div class="feature"><div class="feature-ic"><Icon icon="ri:archive-2-line" width="24" /></div><h3>Big files, whole folders</h3><p>Send up to 50GB and drop entire directory trees with their structure intact.</p></div>
				<div class="feature"><div class="feature-ic"><Icon icon="ri:key-2-line" width="24" /></div><h3>Password protection</h3><p>One toggle locks a file behind a generated key only your recipient holds.</p></div>
				<div class="feature"><div class="feature-ic"><Icon icon="ri:eye-off-line" width="24" /></div><h3>No prying eyes</h3><p>Not us, not your ISP, not the host. The math keeps it shut, not a privacy policy.</p></div>
			</div>
		</div>
	</section>

	<section class="section security">
		<div class="container narrow security-inner">
			<div class="security-ic"><Icon icon="ri:shield-keyhole-line" width="34" /></div>
			<h2 class="section-title">We literally can't read your files.</h2>
			<p>Encryption happens client-side with libsodium. The decryption key is derived from your password and never touches our servers. If you lose it, even we can't recover the file. That is the point.</p>
		</div>
	</section>

	<section class="section cta">
		<div class="container narrow cta-inner">
			<h2 class="section-title">Ready to send something?</h2>
			<p>Drop a file above, or make a free account for 50GB, starred files, and a private dashboard.</p>
			<div class="cta-actions">
				<a href="/auth/signup" class="btn btn-primary btn-lg">Create free account</a>
				<a href="/pricing" class="btn btn-ghost btn-lg">See pricing</a>
			</div>
		</div>
	</section>

	<Footer />
</div>

<div class="bg-effects">
	<div class="glow-spot top"></div>
	<div class="glow-spot bottom"></div>
</div>

<style lang="scss">
	.landing-page { position: relative; z-index: 1; min-height: 100vh; display: flex; flex-direction: column; }

	/* ---------- hero ---------- */
	.hero { display: flex; justify-content: center; padding: clamp(3rem, 9vw, 7rem) var(--gutter) clamp(2.5rem, 6vw, 4rem); }
	.hero-content { width: 100%; max-width: 760px; display: flex; flex-direction: column; align-items: center; text-align: center; gap: var(--space-5); }
	.hero-title { font-size: var(--fs-display); font-weight: var(--fw-black); line-height: var(--lh-tight); letter-spacing: -0.03em; }
	.paw { filter: saturate(1.2); }
	.subtitle { font-size: var(--fs-lg); color: var(--text-secondary); max-width: 56ch; margin: 0 auto; }
	.trust-row { display: flex; flex-wrap: wrap; justify-content: center; gap: var(--space-2); }

	/* ---------- upload zone ---------- */
	.upload-zone { width: 100%; max-width: 620px; margin-top: var(--space-3); background: var(--bg-card); border: 1.5px dashed var(--border-strong); border-radius: var(--radius-lg); padding: clamp(1.5rem, 4vw, 3rem); box-shadow: var(--shadow-lg); transition: border-color var(--dur) var(--ease), background var(--dur) var(--ease), transform var(--dur) var(--ease); cursor: pointer; text-align: left; }
	.upload-zone:hover { border-color: var(--text-muted); }
	.upload-zone.dragging { border-color: var(--primary); background: rgba(255, 70, 85, 0.06); transform: scale(1.01); }

	.upload-placeholder { display: flex; flex-direction: column; align-items: center; gap: var(--space-4); text-align: center; }
	.icon-circle { width: 84px; height: 84px; border-radius: 50%; display: grid; place-items: center; color: var(--primary); background: rgba(255, 70, 85, 0.1); box-shadow: var(--shadow-glow); }
	.upload-placeholder h3 { font-size: var(--fs-h3); }
	.upload-placeholder p { color: var(--text-muted); }
	.browse-btn { display: inline-flex; align-items: center; padding: 0.7rem 1.4rem; background: var(--text-primary); color: var(--bg-app); border-radius: var(--radius-pill); font-weight: var(--fw-semibold); cursor: pointer; transition: transform var(--dur) var(--ease); }
	.browse-btn:hover { transform: translateY(-1px); }
	.limit-badge { display: inline-flex; align-items: center; gap: var(--space-2); margin-top: var(--space-2); padding: 0.35rem 0.8rem; background: var(--tint-soft); border: 1px solid var(--border-default); border-radius: var(--radius-pill); font-size: var(--fs-sm); color: var(--text-secondary); }
	.folder-upload-hint { font-size: var(--fs-sm); }
	.folder-upload-hint .link-btn { color: var(--text-secondary); text-decoration: underline; text-underline-offset: 3px; cursor: pointer; }
	.folder-upload-hint .link-btn:hover { color: var(--text-primary); }

	.file-list { display: flex; flex-direction: column; gap: var(--space-3); width: 100%; }
	.file-item { display: flex; align-items: center; gap: var(--space-4); background: var(--tint-soft); border: 1px solid var(--hairline); padding: var(--space-3); border-radius: var(--radius-sm); }
	.file-item .file-icon { color: var(--primary); font-size: 1.25rem; display: flex; }
	.file-item .file-info { flex: 1; display: flex; flex-direction: column; min-width: 0; }
	.file-item .file-info .name { font-weight: var(--fw-medium); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
	.file-item .file-info .path-hint { font-size: var(--fs-xs); color: var(--text-muted); }
	.file-item .file-info .size { font-size: var(--fs-sm); color: var(--text-secondary); }
	.file-item .remove-btn { background: none; border: none; color: var(--text-muted); cursor: pointer; padding: 4px; border-radius: var(--radius-sm); display: flex; }
	.file-item .remove-btn:hover { background: var(--tint-softer); color: var(--text-primary); }

	.encryption-section { background: var(--tint-soft); border: 1px solid var(--hairline); padding: var(--space-4); border-radius: var(--radius-sm); display: flex; flex-direction: column; gap: var(--space-4); }
	.toggle-row { display: flex; align-items: center; gap: var(--space-3); }
	.toggle-label { font-weight: var(--fw-medium); }
	.password-input-group { display: flex; gap: var(--space-2); }
	.password-field { flex: 1; min-width: 0; background: var(--bg-input); border: 1px solid var(--border-default); color: var(--text-primary); padding: 0.7rem 0.9rem; border-radius: var(--radius-sm); font-family: var(--font-mono); outline: none; }
	.password-field:focus { border-color: var(--primary); }
	.regen-btn, .password-input-group .copy-btn { width: 44px; display: grid; place-items: center; background: var(--tint-softer); border: 1px solid var(--border-default); color: var(--text-primary); border-radius: var(--radius-sm); cursor: pointer; flex: none; }
	.regen-btn:hover, .password-input-group .copy-btn:hover { background: var(--bg-card-hover); }

	.toggle-switch { position: relative; display: inline-block; width: 44px; height: 24px; flex: none; }
	.toggle-switch input { opacity: 0; width: 0; height: 0; }
	.toggle-switch .slider { position: absolute; inset: 0; background: var(--border-strong); transition: 0.3s; border-radius: var(--radius-pill); cursor: pointer; }
	.toggle-switch .slider:before { content: ''; position: absolute; height: 18px; width: 18px; left: 3px; bottom: 3px; background: #fff; transition: 0.3s; border-radius: 50%; }
	.toggle-switch input:checked + .slider { background: var(--primary); }
	.toggle-switch input:checked + .slider:before { transform: translateX(20px); }

	.warning-box { display: flex; align-items: center; gap: var(--space-2); color: var(--warning); font-size: var(--fs-sm); background: rgba(242, 201, 76, 0.1); padding: var(--space-3); border-radius: var(--radius-sm); border: 1px solid rgba(242, 201, 76, 0.25); }

	.upload-action-btn { display: flex; align-items: center; justify-content: center; gap: var(--space-2); background: var(--accent-gradient); color: #fff; border: none; padding: 0.9rem; border-radius: var(--radius-pill); font-weight: var(--fw-semibold); font-size: var(--fs-body); margin-top: var(--space-2); cursor: pointer; box-shadow: 0 6px 20px -6px var(--primary-glow); transition: filter var(--dur) var(--ease); }
	.upload-action-btn:hover { filter: brightness(1.06); }
	.upload-action-btn:disabled { opacity: 0.6; cursor: not-allowed; }
	.add-more-btn { background: none; border: none; color: var(--text-muted); cursor: pointer; font-size: var(--fs-sm); }
	.add-more-btn:hover { color: var(--text-primary); }

	/* ---------- progress ---------- */
	.progress-section { width: 100%; display: flex; flex-direction: column; gap: var(--space-5); margin-top: var(--space-5); background: var(--bg-elevated); padding: var(--space-5); border-radius: var(--radius-md); border: 1px solid var(--border-default); }
	.stats-row { display: flex; justify-content: space-between; padding-bottom: var(--space-3); border-bottom: 1px solid var(--hairline); }
	.stats-row .stat { display: flex; flex-direction: column; gap: var(--space-1); }
	.stats-row .label { font-size: var(--fs-xs); color: var(--text-secondary); text-transform: uppercase; letter-spacing: 0.08em; font-weight: var(--fw-semibold); }
	.stats-row .value { font-size: var(--fs-lg); font-weight: var(--fw-bold); font-family: var(--font-mono); }
	.progress-item { display: flex; flex-direction: column; gap: var(--space-2); }
	.progress-header { display: flex; justify-content: space-between; align-items: center; font-size: var(--fs-sm); }
	.progress-header .label { color: var(--text-primary); font-weight: var(--fw-medium); }
	.progress-header .label.trunc { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; max-width: 200px; }
	.progress-header .percent { color: var(--text-secondary); font-family: var(--font-mono); font-variant-numeric: tabular-nums; }
	.progress-track { width: 100%; height: 10px; background: rgba(0, 0, 0, 0.4); border-radius: var(--radius-pill); overflow: hidden; }
	.progress-item.compact .progress-track { height: 6px; }
	.tiny-label { font-size: var(--fs-xs); color: var(--text-muted); }
	.progress-fill { height: 100%; transition: width 0.3s var(--ease); }
	.progress-fill.total { background: var(--accent-gradient); }
	.progress-fill.file { background: linear-gradient(90deg, #4f46e5, #818cf8); }
	.progress-fill.chunk { background: linear-gradient(90deg, #10b981, #34d399); }

	/* ---------- success modal ---------- */
	.success-overlay { position: fixed; inset: 0; background: rgba(0, 0, 0, 0.6); backdrop-filter: blur(8px); display: flex; align-items: center; justify-content: center; z-index: 50; padding: var(--gutter); }
	.success-card { background: var(--bg-elevated); border: 1px solid var(--border-default); padding: clamp(1.5rem, 4vw, 2.75rem); border-radius: var(--radius-lg); text-align: center; max-width: 500px; width: 100%; box-shadow: var(--shadow-lg); display: flex; flex-direction: column; align-items: center; gap: var(--space-5); }
	.success-icon { color: var(--success); font-size: 3.5rem; display: flex; }
	.success-card h2 { font-size: var(--fs-h2); }
	.success-card p { color: var(--text-secondary); }
	.link-box { display: flex; gap: var(--space-2); background: var(--tint-soft); border: 1px solid var(--hairline); padding: var(--space-3); border-radius: var(--radius-sm); width: 100%; }
	.link-box input { flex: 1; background: transparent; border: none; color: var(--text-secondary); padding: 0 var(--space-2); font-family: var(--font-mono); font-size: var(--fs-sm); outline: none; min-width: 0; }
	.copy-link-btn { display: flex; align-items: center; gap: var(--space-2); background: var(--tint-softer); border: 1px solid var(--border-default); color: var(--text-primary); padding: 0.55rem 1rem; border-radius: var(--radius-sm); cursor: pointer; font-weight: var(--fw-semibold); font-size: var(--fs-sm); }
	.copy-link-btn:hover { background: var(--bg-card-hover); }
	.password-display-box { background: var(--tint-soft); border: 1px solid var(--hairline); border-radius: var(--radius-sm); padding: var(--space-4); text-align: left; width: 100%; }
	.password-label { display: flex; align-items: center; gap: var(--space-2); color: var(--text-secondary); font-size: var(--fs-sm); margin-bottom: var(--space-2); font-weight: var(--fw-medium); }
	.password-value-row { display: flex; align-items: center; justify-content: space-between; gap: var(--space-2); background: var(--bg-input); border-radius: var(--radius-sm); padding: var(--space-2) var(--space-3); border: 1px solid var(--hairline); }
	.password-code { font-family: var(--font-mono); font-size: var(--fs-lg); color: var(--primary); word-break: break-all; }
	.password-value-row .copy-btn { background: transparent; border: none; color: var(--text-secondary); cursor: pointer; display: flex; }
	.password-value-row .copy-btn:hover { color: var(--text-primary); }
	.password-hint { margin-top: var(--space-2); font-size: var(--fs-xs); color: var(--text-muted); }
	.action-buttons { width: 100%; }
	.action-buttons .primary-btn { width: 100%; background: var(--accent-gradient); color: #fff; border: none; padding: 0.9rem; border-radius: var(--radius-pill); font-weight: var(--fw-semibold); font-size: var(--fs-body); cursor: pointer; transition: filter var(--dur) var(--ease); }
	.action-buttons .primary-btn:hover { filter: brightness(1.06); }

	/* ---------- sections ---------- */
	.section-title { font-size: var(--fs-h2); margin-top: var(--space-2); }
	.how .container, .features .container { display: flex; flex-direction: column; gap: var(--space-6); }

	.steps { display: grid; grid-template-columns: repeat(3, 1fr); gap: var(--space-5); }
	.step { background: var(--bg-card); border: 1px solid var(--border-default); border-radius: var(--radius-md); padding: var(--space-6); display: flex; flex-direction: column; gap: var(--space-3); }
	.step-n { font-family: var(--font-mono); font-size: var(--fs-sm); color: var(--primary); font-weight: var(--fw-semibold); }
	.step-ic { width: 52px; height: 52px; border-radius: var(--radius-sm); display: grid; place-items: center; color: var(--primary); background: rgba(255, 70, 85, 0.1); }
	.step p { color: var(--text-secondary); }

	.feature-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(260px, 1fr)); gap: var(--space-5); }
	.feature { background: var(--bg-card); border: 1px solid var(--border-default); border-radius: var(--radius-md); padding: var(--space-5); display: flex; flex-direction: column; gap: var(--space-3); transition: border-color var(--dur) var(--ease), transform var(--dur) var(--ease); }
	.feature:hover { border-color: var(--border-strong); transform: translateY(-2px); }
	.feature-ic { width: 46px; height: 46px; border-radius: var(--radius-sm); display: grid; place-items: center; color: var(--primary); background: rgba(255, 70, 85, 0.1); }
	.feature p { color: var(--text-secondary); font-size: var(--fs-sm); }

	.security-inner { text-align: center; display: flex; flex-direction: column; align-items: center; gap: var(--space-4); padding: clamp(2rem, 5vw, 3.5rem); background: linear-gradient(180deg, rgba(255, 70, 85, 0.07), var(--tint-soft)); border: 1px solid var(--border-default); border-radius: var(--radius-lg); }
	.security-ic { width: 70px; height: 70px; border-radius: var(--radius-md); display: grid; place-items: center; color: var(--primary); background: rgba(255, 70, 85, 0.12); box-shadow: var(--shadow-glow); }
	.security-inner p { color: var(--text-secondary); max-width: 60ch; }

	.cta-inner { text-align: center; display: flex; flex-direction: column; align-items: center; gap: var(--space-5); }
	.cta-inner p { color: var(--text-secondary); max-width: 52ch; }
	.cta-actions { display: flex; flex-wrap: wrap; gap: var(--space-3); justify-content: center; }

	/* ---------- background fx ---------- */
	.bg-effects { position: fixed; inset: 0; z-index: 0; pointer-events: none; overflow: hidden; }
	.glow-spot { position: absolute; width: 600px; height: 600px; filter: blur(110px); border-radius: 50%; }
	.glow-spot.top { top: -22%; left: 12%; background: radial-gradient(circle, rgba(255, 70, 85, 0.16) 0%, transparent 70%); }
	.glow-spot.bottom { bottom: -25%; right: 10%; background: radial-gradient(circle, rgba(74, 163, 226, 0.1) 0%, transparent 70%); }

	/* ---------- responsive ---------- */
	@media (max-width: 860px) {
		.steps { grid-template-columns: 1fr; }
	}
	@media (max-width: 768px) {
		.progress-header .label.trunc { max-width: 140px; }
		.link-box { flex-direction: column; }
	}
</style>
