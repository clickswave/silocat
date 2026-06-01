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
			toast.error('Upload failed or interrupted');
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
			<h1>Moving mountains of data,<br /><span class="text-gradient">securely.</span></h1>
			<p class="subtitle">
				Kitty powered E2E encrypted anonymous file-sharing and cloud storage platform with parallel
				downloads.
			</p>

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

	<Footer />
</div>

<div class="bg-effects">
	<div class="glow-spot top"></div>
	<div class="glow-spot bottom"></div>
</div>

<style lang="scss">
	:global(body) {
		margin: 0;
		background-color: #0b0b0d;
		color: white;
		font-family: 'Outfit', sans-serif;
	}

	.landing-page {
		min-height: 100vh;
		position: relative;
		overflow-y: auto; /* Enable scrolling if needed, though min-height usually handles naturally */
		display: flex;
		flex-direction: column;
	}

	.hero {
		flex: 1;
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 10;
		padding: 2rem;
	}

	.hero-content {
		text-align: center;
		max-width: 800px;
		width: 100%;
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 1.5rem;

		h1 {
			font-size: 4rem; // Large hero text
			font-weight: 800;
			margin: 0;
			line-height: 1.1;
			letter-spacing: -0.02em;

			.text-gradient {
				background: linear-gradient(135deg, #fff 0%, #a1a1aa 100%);
				-webkit-background-clip: text;
				-webkit-text-fill-color: transparent;
			}
		}

		.subtitle {
			font-size: 1.25rem;
			color: #a1a1aa;
			margin-bottom: 2rem;
		}

		.upload-zone {
			background: rgba(20, 20, 22, 0.6);
			backdrop-filter: blur(20px);
			border: 2px dashed rgba(255, 255, 255, 0.1);
			border-radius: 24px;
			padding: 3rem;
			width: 100%;
			max-width: 600px;
			transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
			cursor: pointer;
			box-shadow: 0 20px 40px rgba(0, 0, 0, 0.3);

			&.dragging {
				border-color: var(--primary, #ff4655);
				background: rgba(255, 70, 85, 0.05);
				transform: scale(1.02);
			}

			&:hover {
				border-color: rgba(255, 255, 255, 0.2);
			}

			.upload-placeholder {
				display: flex;
				flex-direction: column;
				align-items: center;
				gap: 1rem;

				.icon-circle {
					width: 80px;
					height: 80px;
					background: rgba(255, 255, 255, 0.05);
					border-radius: 50%;
					display: flex;
					align-items: center;
					justify-content: center;
					color: var(--primary, #ff4655);
					margin-bottom: 0.5rem;
				}

				h3 {
					margin: 0;
					font-size: 1.5rem;
					font-weight: 600;
				}

				p {
					color: #71717a;
					margin: 0;
				}

				.browse-btn {
					background: white;
					color: black;
					padding: 0.75rem 1.5rem;
					border-radius: 8px;
					font-weight: 600;
					cursor: pointer;
					transition: transform 0.1s;
					display: inline-block;

					&:hover {
						transform: scale(1.05);
					}
				}

				.limit-badge {
					margin-top: 1rem;
					background: rgba(255, 255, 255, 0.05);
					padding: 0.4rem 0.8rem;
					border-radius: 20px;
					font-size: 0.85rem;
					color: #a1a1aa;
					display: flex;
					align-items: center;
					gap: 0.4rem;
				}

				.folder-upload-hint {
					margin-top: 0.5rem;
					font-size: 0.9rem;

					.link-btn {
						color: #a1a1aa;
						text-decoration: underline;
						cursor: pointer;
						&:hover {
							color: white;
						}
					}
				}
			}

			.file-list {
				display: flex;
				flex-direction: column;
				gap: 0.75rem;
				width: 100%;

				.file-item {
					display: flex;
					align-items: center;
					background: rgba(255, 255, 255, 0.05);
					padding: 0.75rem;
					border-radius: 12px;
					gap: 1rem;

					.file-icon {
						color: var(--primary, #ff4655);
						font-size: 1.25rem;
					}

					.file-info {
						flex: 1;
						display: flex;
						flex-direction: column;

						.name {
							font-weight: 500;
						}

						.path-hint {
							font-size: 0.75rem;
							color: #71717a;
						}

						.size {
							font-size: 0.85rem;
							color: #a1a1aa;
						}
					}

					.remove-btn {
						background: none;
						border: none;
						color: #71717a;
						cursor: pointer;
						padding: 4px;
						border-radius: 4px;

						&:hover {
							background: rgba(255, 255, 255, 0.1);
							color: white;
						}
					}
				}

				.encryption-section {
					background: rgba(255, 255, 255, 0.05);
					padding: 1rem;
					border-radius: 12px;
					display: flex;
					flex-direction: column;
					gap: 1rem;
					margin-top: 0.5rem;

					.toggle-row {
						display: flex;
						align-items: center;
						gap: 0.75rem;

						.toggle-label {
							font-weight: 500;
							font-size: 0.95rem;
						}
					}

					.password-input-group {
						display: flex;
						gap: 0.5rem;

						.password-field {
							flex: 1;
							background: rgba(0, 0, 0, 0.3);
							border: 1px solid rgba(255, 255, 255, 0.1);
							color: white;
							padding: 0.75rem 1rem;
							border-radius: 8px;
							font-family: monospace;
							font-size: 1rem;
							outline: none;

							&:focus {
								border-color: var(--primary, #ff4655);
							}
						}

						.regen-btn,
						.copy-btn {
							background: rgba(255, 255, 255, 0.1);
							border: none;
							color: white;
							width: 42px;
							border-radius: 8px;
							cursor: pointer;
							display: flex;
							align-items: center;
							justify-content: center;

							&:hover {
								background: rgba(255, 255, 255, 0.2);
							}
						}
					}
				}

				/* Switch Styling */
				.toggle-switch {
					position: relative;
					display: inline-block;
					width: 44px;
					height: 24px;

					input {
						opacity: 0;
						width: 0;
						height: 0;
					}

					.slider {
						position: absolute;
						cursor: pointer;
						top: 0;
						left: 0;
						right: 0;
						bottom: 0;
						background-color: #3a3a3c;
						transition: 0.4s;
						border-radius: 24px;

						&:before {
							position: absolute;
							content: '';
							height: 18px;
							width: 18px;
							left: 3px;
							bottom: 3px;
							background-color: white;
							transition: 0.4s;
							border-radius: 50%;
						}
					}

					input:checked + .slider {
						background-color: var(--primary, #ff4655);
					}

					input:checked + .slider:before {
						transform: translateX(20px);
					}
				}

				.warning-box {
					display: flex;
					align-items: center;
					gap: 0.5rem;
					color: #fbaceb; /* Amber/Warning Color */
					font-size: 0.9rem;
					background: rgba(251, 172, 235, 0.1);
					padding: 0.75rem;
					border-radius: 8px;
					border: 1px solid rgba(251, 172, 235, 0.2);
					margin-top: 0.5rem;
				}

				.upload-action-btn {
					background: var(--primary, #ff4655);
					color: white;
					border: none;
					padding: 1rem;
					border-radius: 12px;
					font-weight: 600;
					font-size: 1rem;
					margin-top: 1rem;
					cursor: pointer;
					display: flex;
					align-items: center;
					justify-content: center;
					gap: 0.5rem;

					&:hover {
						background: #e03e4b;
					}
				}

				.add-more-btn {
					background: transparent;
					border: none;
					color: #71717a;
					cursor: pointer;
					font-size: 0.9rem;

					&:hover {
						color: white;
					}
				}
			}
			/* Upload Zone Responsive Fixes */
			@media (max-width: 768px) {
				/* Assuming the main upload zone container is the parent of .file-item, etc. */
				/* This targets the container that holds all the upload form elements */
				padding: 1.5rem !important; /* Force override if specificity issue */
				border-radius: 16px !important;

				.file-item {
					padding: 0.65rem;
					gap: 0.75rem;
				}

				.encryption-section {
					padding: 0.85rem;
					gap: 0.85rem;
				}

				.encryption-section .password-input-group .password-field {
					padding: 0.65rem 0.85rem;
					font-size: 0.9rem;
				}

				.encryption-section .password-input-group .regen-btn,
				.encryption-section .password-input-group .copy-btn {
					width: 38px;
					height: 38px;
				}

				.upload-action-btn {
					padding: 0.8rem;
					font-size: 0.95rem;
				}
			}
		}

		.success-overlay {
			position: fixed;
			inset: 0;
			background: rgba(0, 0, 0, 0.6);
			backdrop-filter: blur(8px);
			display: flex;
			align-items: center;
			justify-content: center;
			z-index: 50;
			padding: 2rem;

			.success-card {
				background: rgba(20, 20, 22, 0.85);
				backdrop-filter: blur(24px);
				border: 1px solid rgba(255, 255, 255, 0.1);
				padding: 3rem;
				border-radius: 24px;
				text-align: center;
				max-width: 500px;
				width: 100%;
				box-shadow: 0 25px 50px -12px rgba(0, 0, 0, 0.5);
				display: flex;
				flex-direction: column;
				align-items: center;
				gap: 1.5rem;

				/* Mobile Optimization */
				@media (max-width: 768px) {
					padding: 1.5rem;
					max-width: 90%;
					width: auto;
					border-radius: 20px;
					gap: 1rem;
				}

				.success-icon {
					color: #22c55e;
					font-size: 4rem;
					display: flex;

					@media (max-width: 768px) {
						font-size: 3rem;
					}
				}

				h2 {
					font-size: 1.8rem;
					font-weight: 700;
					margin: 0;
					color: white;

					@media (max-width: 768px) {
						font-size: 1.4rem;
					}
				}

				p {
					color: #a1a1aa;
					font-size: 1rem;
					margin: 0;
				}

				.link-box {
					display: flex;
					gap: 0.5rem;
					background: rgba(255, 255, 255, 0.05);
					padding: 0.75rem;
					border-radius: 12px;
					width: 100%;
					margin-top: 0.5rem;

					@media (max-width: 768px) {
						flex-direction: column;
						gap: 0.8rem;
					}

					input {
						flex: 1;
						background: transparent;
						border: none;
						color: #a1a1aa;
						padding: 0 0.5rem;
						font-family: monospace;
						font-size: 0.95rem;
						outline: none;

						@media (max-width: 768px) {
							width: 100%;
							text-align: center;
							padding: 0.5rem 0;
							border-bottom: 1px solid rgba(255, 255, 255, 0.1);
						}
					}

					.copy-link-btn {
						background: rgba(255, 255, 255, 0.1);
						border: none;
						color: white;
						padding: 0.6rem 1rem;
						border-radius: 8px;
						cursor: pointer;
						display: flex;
						align-items: center;
						gap: 0.5rem;
						font-weight: 600;
						transition: all 0.2s;
						font-size: 0.9rem;
						justify-content: center; /* Center content especially for mobile stack */

						&:hover {
							background: rgba(255, 255, 255, 0.2);
						}
					}
				}

				.password-display-box {
					background: rgba(255, 255, 255, 0.03);
					border: 1px solid rgba(255, 255, 255, 0.1);
					border-radius: 12px;
					padding: 1rem;
					margin-top: 1rem;
					text-align: left;
					width: 100%;
					box-sizing: border-box;

					.password-label {
						display: flex;
						align-items: center;
						gap: 0.5rem;
						color: #a1a1aa;
						font-size: 0.9rem;
						margin-bottom: 0.5rem;
						font-weight: 500;
					}

					.password-value-row {
						display: flex;
						align-items: center;
						justify-content: space-between;
						background: rgba(0, 0, 0, 0.3);
						border-radius: 8px;
						padding: 0.5rem 0.75rem;
						border: 1px solid rgba(255, 255, 255, 0.05);

						.password-code {
							font-family: 'JetBrains Mono', monospace;
							font-size: 1.1rem;
							color: var(--primary, #ff4655);
							letter-spacing: 0.05em;
							/* Ensure long passwords break or scroll on mobile */
							overflow-wrap: break-word;
							word-break: break-all;
						}

						.copy-btn {
							background: transparent;
							border: none;
							color: #a1a1aa;
							cursor: pointer;
							transition: color 0.2s;
							padding: 4px;
							display: flex;
							align-items: center;
							justify-content: center;
							font-size: 1.2rem;

							&:hover {
								color: white;
							}
						}
					}

					.password-hint {
						margin: 0.5rem 0 0 0;
						font-size: 0.8rem;
						color: #71717a;
					}
				}

				.action-buttons {
					width: 100%;
					.primary-btn {
						background: var(--primary, #ff4655);
						color: white;
						border: none;
						width: 100%;
						padding: 1rem;
						border-radius: 12px;
						font-weight: 600;
						font-size: 1rem;
						cursor: pointer;
						transition: all 0.2s;

						&:hover {
							background: #e03e4b;
							transform: translateY(-2px);
							box-shadow: 0 4px 12px rgba(224, 62, 75, 0.3);
						}
						@media (max-width: 768px) {
							padding: 0.8rem;
						}
					}
				}
			}
		}
	}

	.bg-effects {
		position: fixed; /* Fixed so it stays while scrolling */
		inset: 0;
		z-index: 0;
		pointer-events: none;

		.glow-spot {
			position: absolute;
			width: 600px;
			height: 600px;
			background: radial-gradient(circle, rgba(255, 70, 85, 0.15) 0%, transparent 70%);
			filter: blur(100px);

			&.top {
				top: -20%;
				left: 20%;
			}

			&.bottom {
				bottom: -20%;
				right: 20%;
				background: radial-gradient(circle, rgba(50, 50, 255, 0.1) 0%, transparent 70%);
			}
		}
	}

	.progress-section {
		width: 100%;
		display: flex;
		flex-direction: column;
		gap: 1.25rem;
		margin-top: 1.5rem;
		background: rgba(20, 20, 22, 0.6);
		backdrop-filter: blur(12px);
		padding: 1.5rem;
		border-radius: 16px;
		border: 1px solid rgba(255, 255, 255, 0.08);
		box-shadow: 0 10px 40px rgba(0, 0, 0, 0.2);

		.stats-row {
			display: flex;
			justify-content: space-between;
			margin-bottom: 0.25rem;
			padding-bottom: 0.75rem;
			border-bottom: 1px solid rgba(255, 255, 255, 0.05);

			.stat {
				display: flex;
				flex-direction: column;
				gap: 0.35rem;

				.label {
					font-size: 0.7rem;
					color: #a1a1aa;
					text-transform: uppercase;
					letter-spacing: 0.08em;
					font-weight: 600;
				}
				.value {
					font-size: 1.1rem;
					font-weight: 700;
					color: white;
					text-shadow: 0 0 20px rgba(255, 255, 255, 0.3);
					font-family: 'JetBrains Mono', monospace;
				}
			}
		}

		.progress-item {
			display: flex;
			flex-direction: column;
			gap: 0.6rem;

			&.compact {
				gap: 0.3rem;
				margin-top: 0.25rem;

				.progress-header {
					.label {
						font-size: 0.75rem;
						color: #71717a;
					}
					.percent {
						font-size: 0.75rem;
					}
				}

				.progress-track {
					height: 6px;
				}
			}

			.progress-header {
				display: flex;
				justify-content: space-between;
				align-items: center;
				font-size: 0.9rem;

				.label {
					color: #e4e4e7;
					font-weight: 500;

					&.trunc {
						white-space: nowrap;
						overflow: hidden;
						text-overflow: ellipsis;
						max-width: 200px;
					}
				}
				.percent {
					color: #a1a1aa;
					font-variant-numeric: tabular-nums;
					font-family: 'JetBrains Mono', monospace;
					font-size: 0.85rem;
				}
			}

			.progress-track {
				width: 100%;
				height: 10px;
				background: rgba(0, 0, 0, 0.4);
				border-radius: 6px;
				overflow: hidden;
				position: relative;
				box-shadow: inset 0 2px 4px rgba(0, 0, 0, 0.3);

				.progress-fill {
					height: 100%;
					transition: width 0.3s cubic-bezier(0.4, 0, 0.2, 1);
					position: relative;

					// Glow overflow
					&::after {
						content: '';
						position: absolute;
						top: 0;
						right: 0;
						bottom: 0;
						width: 10px;
						background: white;
						filter: blur(4px);
						opacity: 0.5;
					}

					&.total {
						background: linear-gradient(90deg, #ff4655, #ff8090);
						box-shadow: 0 0 15px rgba(255, 70, 85, 0.5);
					}
					&.file {
						background: linear-gradient(90deg, #4f46e5, #818cf8);
						box-shadow: 0 0 15px rgba(79, 70, 229, 0.5);
					}
					&.chunk {
						background: linear-gradient(90deg, #10b981, #34d399);
						box-shadow: 0 0 15px rgba(16, 185, 129, 0.5);
					}
				}
			}
		}
	}
</style>
