<script>
	import Icon from '@iconify/svelte';
	import Seo from '$lib/components/Seo.svelte';
	import { softwareApplicationSchema } from '$lib/seo.js';
	import { fade } from 'svelte/transition';
	import Navbar from '$lib/components/Navbar.svelte';
	import Footer from '$lib/components/Footer.svelte';
	import { Button, Switch, PasswordInput, Progress, Modal, Copy } from '$lib/ui';

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

	function onEncryptionToggle() {
		if (encryptionEnabled && !password) {
			password = generatePassword();
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

			if (successCount > 0) toast.success(`Upload complete (${successCount} files)`);
		} catch (e) {
			console.error('Batch upload interrupted', e);
			const d = e?.response?.data;
			if (e?.response?.status === 403 && (d?.banned || /banned/i.test(d?.error || ''))) {
				toast.error(d?.error || 'You are banned from using Silocat.');
			} else {
				toast.error('Upload failed or interrupted');
			}
		} finally {
			isUploading = false;
		}
	}

	function resetUpload() {
		uploadSuccessUrl = null;
		showSuccessModal = false;
		files = [];
		password = '';
		encryptionEnabled = false;
	}

	function fmtSize(bytes) {
		if (bytes >= 1024 * 1024 * 1024) return (bytes / (1024 * 1024 * 1024)).toFixed(2) + ' GB';
		return (bytes / (1024 * 1024)).toFixed(2) + ' MB';
	}
</script>

<Seo
	title="Silocat: End-to-end encrypted file sharing & cloud storage"
	description="Zero-knowledge, end-to-end encrypted file sharing and cloud storage. Upload up to 20 GB, share an anonymous link with parallel downloads, and keep full control of your data."
	keywords="encrypted file sharing, secure cloud storage, zero knowledge storage, end-to-end encryption, anonymous file share, send large files, private file sharing"
	schema={softwareApplicationSchema()}
/>

<div class="landing-page">
	<Navbar />

	<main class="hero">
		<div class="hero-content">
			<h1 class="hero-title">Big files. Zero knowledge.</h1>
			<p class="subtitle">
				End-to-end encrypted file transfer. No account needed. Up to 20&nbsp;GB free.
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
						<Icon icon="ri:upload-cloud-2-line" width="28" class="drop-ic" />
						<label for="file-upload" class="drop-line">
							Drop files or <span class="browse">browse</span>
						</label>
						<input type="file" id="file-upload" multiple onchange={handleFileSelect} hidden />
						<p class="drop-hint">
							encrypted before they leave
							<span class="dot">·</span>
							<label for="folder-upload" class="folder-link">upload a folder</label>
						</p>
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
				{:else}
					<div class="file-list">
						{#each files as fileItem, i (i)}
							{@const f = fileItem.file || fileItem}
							{@const path = fileItem.path || ''}
							<div class="file-item" transition:fade={{ duration: 120 }}>
								<Icon icon="ri:file-3-line" width="16" class="file-ic" />
								<div class="file-info">
									<span class="name">{f.name}</span>
									{#if path}<span class="path-hint">{path}/</span>{/if}
								</div>
								<span class="size">{fmtSize(f.size)}</span>
								<button class="remove-btn" onclick={() => removeFile(i)} aria-label="Remove">
									<Icon icon="ri:close-line" width="15" />
								</button>
							</div>
						{/each}

						<div class="encrypt-row">
							<Switch
								bind:checked={encryptionEnabled}
								label="Password protect"
								disabled={isUploading}
								onchange={onEncryptionToggle}
							/>
							{#if !encryptionEnabled}
								<span class="plain-note">uploads unencrypted</span>
							{/if}
						</div>

						{#if encryptionEnabled}
							<div transition:fade={{ duration: 120 }}>
								<PasswordInput
									bind:value={password}
									placeholder="Password"
									copyable
									generatable
									disabled={isUploading}
								/>
							</div>
						{/if}

						{#if isUploading}
							<div class="progress-stack" transition:fade={{ duration: 120 }}>
								<div class="progress-row">
									<span class="p-label">Total</span>
									<Progress value={uploadStats.totalProgress} size="md" />
									<span class="p-pct">{Math.round(uploadStats.totalProgress)}%</span>
								</div>
								<div class="progress-row">
									<span class="p-label trunc">{uploadStats.currentFileName || 'Processing'}</span>
									<Progress value={uploadStats.fileProgress} size="sm" tone="neutral" />
									<span class="p-pct">{Math.round(uploadStats.fileProgress)}%</span>
								</div>
								<div class="p-stats">
									<span>{uploadStats.speed ? (uploadStats.speed / 1024 / 1024).toFixed(1) + ' MB/s' : 'starting'}</span>
									<span>{uploadStats.eta ? 'eta ' + Math.ceil(uploadStats.eta) + 's' : ''}</span>
								</div>
							</div>
						{/if}

						<div class="upload-actions">
							<Button
								block
								loading={isUploading || uploadMutation.isPending}
								onclick={startUpload}
							>
								{isUploading || uploadMutation.isPending ? 'Uploading' : 'Upload'}
							</Button>
							{#if !isUploading}
								<Button variant="quiet" size="sm" onclick={() => (files = [])}>Clear all</Button>
							{/if}
						</div>
					</div>
				{/if}
			</div>

			<p class="steps-line">
				drop it <span class="dot">·</span> we encrypt it <span class="dot">·</span> share the link
			</p>
		</div>
	</main>

	<section class="section facts">
		<div class="container narrow">
			<div class="fact">
				<h3>Zero-knowledge</h3>
				<p>Files are encrypted in your browser with libsodium before upload. We store ciphertext, nothing else. If you lose the password, even we can't recover the file. That is the point.</p>
			</div>
			<div class="fact">
				<h3>Anonymous</h3>
				<p>No email, no signup, no tracking. A local browser key lets you manage your uploads across sessions without an account.</p>
			</div>
			<div class="fact">
				<h3>Fast</h3>
				<p>Chunked storage downloads in parallel and saturates the connection instead of trickling one stream.</p>
			</div>
			<div class="fact">
				<h3>Whole folders</h3>
				<p>Drop entire directory trees with their structure intact. Up to 20Up to 20&nbsp;GB anonymously, 50&nbsp;GB with a free account.nbsp;GB anonymously, 10Up to 20&nbsp;GB anonymously, 50&nbsp;GB with a free account.nbsp;GB with a free account.</p>
			</div>
		</div>
	</section>

	<section class="section oss">
		<div class="container narrow oss-inner">
			<p>
				Silocat is open source under AGPL-3.0.
				<a href="https://github.com/clickswave/silocat" target="_blank" rel="noreferrer">Read the code</a>, or self-host it.
			</p>
		</div>
	</section>

	<section class="section cta">
		<div class="container narrow cta-inner">
			<h2>Ready to send something?</h2>
			<div class="cta-actions">
				<Button size="lg" href="/auth/signup">Create free account</Button>
				<Button size="lg" variant="ghost" href="/pricing">See pricing</Button>
			</div>
		</div>
	</section>

	<Footer />
</div>

<Modal
	open={showSuccessModal}
	title={isUploading ? 'Uploading' : 'Ready to share'}
	onclose={() => {
		if (!isUploading) resetUpload();
	}}
>
	<div class="success-stack">
		<p class="success-line">
			{#if isUploading}
				The link works already. Anyone opening it will see the files arrive.
			{:else}
				Anyone with this link can download {files.length > 1 ? 'these files' : 'this file'}.
			{/if}
		</p>

		{#if uploadSuccessUrl}
			<div class="link-box">
				<input type="text" readonly value={uploadSuccessUrl} onclick={(e) => e.target.select()} />
				<Copy text={uploadSuccessUrl} label="Copy link" />
			</div>
		{/if}

		{#if isUploading}
			<div class="progress-stack">
				<div class="progress-row">
					<span class="p-label">Total</span>
					<Progress value={uploadStats.totalProgress} size="md" />
					<span class="p-pct">{Math.round(uploadStats.totalProgress)}%</span>
				</div>
				<div class="progress-row">
					<span class="p-label trunc">{uploadStats.currentFileName || 'Processing'}</span>
					<Progress value={uploadStats.fileProgress} size="sm" tone="neutral" />
					<span class="p-pct">{Math.round(uploadStats.fileProgress)}%</span>
				</div>
				<div class="p-stats">
					<span>{uploadStats.speed ? (uploadStats.speed / 1024 / 1024).toFixed(1) + ' MB/s' : 'starting'}</span>
					<span>{uploadStats.eta ? 'eta ' + Math.ceil(uploadStats.eta) + 's' : ''}</span>
				</div>
			</div>
		{/if}

		{#if encryptionEnabled && password}
			<div class="password-box">
				<span class="pw-label">Decryption password, save it now. It is required to download.</span>
				<div class="pw-row">
					<code>{password}</code>
					<Copy text={password} label="Copy password" size="sm" />
				</div>
			</div>
		{/if}
	</div>

	{#snippet footer()}
		<Button block disabled={isUploading} onclick={resetUpload}>
			{isUploading ? 'Uploading in background' : 'Upload another'}
		</Button>
	{/snippet}
</Modal>

<style lang="scss">
	.landing-page {
		min-height: 100vh;
		display: flex;
		flex-direction: column;
	}

	/* ---------- hero ---------- */
	.hero {
		display: flex;
		justify-content: center;
		padding: clamp(3.5rem, 10vw, 7rem) var(--gutter) clamp(2rem, 5vw, 3.5rem);
	}
	.hero-content {
		width: 100%;
		max-width: 640px;
		display: flex;
		flex-direction: column;
		align-items: center;
		text-align: center;
		gap: var(--space-4);
	}
	.hero-title {
		font-size: var(--fs-display);
		font-weight: var(--fw-black);
		letter-spacing: -0.03em;
	}
	.subtitle {
		font-size: var(--fs-lg);
		color: var(--ink-mute);
		margin: 0;
	}

	/* ---------- upload zone ---------- */
	.upload-zone {
		width: 100%;
		margin-top: var(--space-5);
		background: var(--surface);
		border: 1px dashed var(--edge-strong);
		border-radius: var(--radius-lg);
		padding: clamp(1.25rem, 3vw, 2rem);
		transition:
			border-color var(--dur) var(--ease),
			background var(--dur) var(--ease);
		text-align: left;

		&.dragging {
			border-color: var(--accent);
			background: var(--accent-soft);
		}
	}

	.upload-placeholder {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: var(--space-3);
		text-align: center;
		padding: var(--space-6) 0;
		color: var(--ink-faint);

		:global(.drop-ic) {
			color: var(--ink-faint);
		}
	}
	.drop-line {
		font-size: var(--fs-lg);
		font-weight: var(--fw-medium);
		color: var(--ink);
		cursor: pointer;

		.browse {
			color: var(--accent);
			text-decoration: underline;
			text-underline-offset: 3px;
		}
	}
	.drop-hint {
		font-size: var(--fs-sm);
		color: var(--ink-faint);
		margin: 0;

		.folder-link {
			cursor: pointer;
			text-decoration: underline;
			text-underline-offset: 3px;
			&:hover {
				color: var(--ink);
			}
		}
	}
	.dot {
		margin-inline: var(--space-1);
		color: var(--ink-faint);
	}

	/* ---------- file list ---------- */
	.file-list {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
		width: 100%;
	}
	.file-item {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		padding: var(--space-2) var(--space-3);
		border: 1px solid var(--edge);
		border-radius: var(--radius-sm);

		:global(.file-ic) {
			color: var(--ink-faint);
			flex-shrink: 0;
		}
	}
	.file-info {
		flex: 1;
		display: flex;
		align-items: baseline;
		gap: var(--space-2);
		min-width: 0;

		.name {
			font-size: var(--fs-sm);
			font-weight: var(--fw-medium);
			white-space: nowrap;
			overflow: hidden;
			text-overflow: ellipsis;
		}
		.path-hint {
			font-size: var(--fs-xs);
			color: var(--ink-faint);
			flex-shrink: 0;
		}
	}
	.size {
		font-family: var(--font-mono);
		font-size: var(--fs-xs);
		color: var(--ink-faint);
		flex-shrink: 0;
	}
	.remove-btn {
		display: flex;
		background: none;
		border: none;
		color: var(--ink-faint);
		cursor: pointer;
		padding: 4px;
		border-radius: var(--radius-sm);

		&:hover {
			background: var(--tint-soft);
			color: var(--ink);
		}
	}

	.encrypt-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-3);
		padding-top: var(--space-2);
	}
	.plain-note {
		font-size: var(--fs-xs);
		color: var(--warn);
	}

	.upload-actions {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: var(--space-2);
		margin-top: var(--space-2);
	}

	/* ---------- progress ---------- */
	.progress-stack {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
		padding-top: var(--space-2);
	}
	.progress-row {
		display: grid;
		grid-template-columns: 90px 1fr 42px;
		align-items: center;
		gap: var(--space-3);
	}
	.p-label {
		font-size: var(--fs-xs);
		color: var(--ink-mute);

		&.trunc {
			white-space: nowrap;
			overflow: hidden;
			text-overflow: ellipsis;
		}
	}
	.p-pct {
		font-family: var(--font-mono);
		font-size: var(--fs-xs);
		color: var(--ink-mute);
		text-align: right;
		font-variant-numeric: tabular-nums;
	}
	.p-stats {
		display: flex;
		justify-content: space-between;
		font-family: var(--font-mono);
		font-size: var(--fs-xs);
		color: var(--ink-faint);
	}

	/* ---------- steps line ---------- */
	.steps-line {
		margin-top: var(--space-5);
		font-family: var(--font-mono);
		font-size: var(--fs-sm);
		color: var(--ink-faint);
	}

	/* ---------- facts ---------- */
	.facts .container {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: var(--space-8) var(--space-8);
	}
	.fact {
		border-top: 1px solid var(--edge);
		padding-top: var(--space-4);

		h3 {
			font-size: var(--fs-body);
			font-weight: var(--fw-semibold);
			margin-bottom: var(--space-2);
		}
		p {
			font-size: var(--fs-sm);
			color: var(--ink-mute);
			line-height: var(--lh-normal);
		}
	}

	/* ---------- oss ---------- */
	.oss {
		padding-block: 0;
	}
	.oss-inner {
		text-align: center;

		p {
			font-size: var(--fs-sm);
			color: var(--ink-faint);
		}
		a {
			color: var(--ink-mute);
			text-decoration: underline;
			text-underline-offset: 3px;
			&:hover {
				color: var(--ink);
			}
		}
	}

	/* ---------- cta ---------- */
	.cta-inner {
		text-align: center;
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: var(--space-5);
	}
	.cta-actions {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-3);
		justify-content: center;
	}

	/* ---------- success modal ---------- */
	.success-stack {
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
	}
	.success-line {
		color: var(--ink-mute);
		font-size: var(--fs-sm);
		margin: 0;
	}
	.link-box {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		background: var(--bg);
		border: 1px solid var(--edge);
		border-radius: var(--radius-sm);
		padding: var(--space-1) var(--space-1) var(--space-1) var(--space-3);

		input {
			flex: 1;
			min-width: 0;
			background: transparent;
			border: none;
			outline: none;
			color: var(--ink);
			font-family: var(--font-mono);
			font-size: var(--fs-sm);
			padding: var(--space-2) 0;
		}
	}
	.password-box {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);

		.pw-label {
			font-size: var(--fs-xs);
			color: var(--warn);
		}
		.pw-row {
			display: flex;
			align-items: center;
			justify-content: space-between;
			gap: var(--space-2);
			background: var(--bg);
			border: 1px solid var(--edge);
			border-radius: var(--radius-sm);
			padding: var(--space-2) var(--space-2) var(--space-2) var(--space-3);

			code {
				font-size: var(--fs-body);
				color: var(--ink);
				word-break: break-all;
			}
		}
	}

	/* ---------- responsive ---------- */
	@media (max-width: 720px) {
		.facts .container {
			grid-template-columns: 1fr;
			gap: var(--space-6);
		}
	}
</style>
