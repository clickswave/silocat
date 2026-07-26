<script>
	import Icon from '$lib/ui/Icon.svelte';
	import Seo from '$lib/components/Seo.svelte';
	import { softwareApplicationSchema } from '$lib/seo.js';
	import { fade } from 'svelte/transition';
	import Navbar from '$lib/components/Navbar.svelte';
	import Footer from '$lib/components/Footer.svelte';
	import { Button, Modal } from '$lib/ui';
	import { generatePassword } from '$lib/password.js';

	import { createMutation } from '@tanstack/svelte-query';
	import { toast } from '$lib/toast.js';
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

	// Password protection defaults ON: the safe choice should be the default
	// one, not an extra step. A password is generated up front so the field is
	// never empty when the switch is already on.
	let encryptionEnabled = $state(true);
	let password = $state('');
	let showPassword = $state(false);

	function onEncryptionToggle() {
		if (encryptionEnabled && !password) password = generatePassword();
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
	// --- zone state machine: idle -> dragging -> staged -> uploading ---------
	let zone = $derived(
		isUploading ? 'uploading' : files.length > 0 ? 'staged' : isDragging ? 'dragging' : 'idle'
	);

	// Drag events fire per-child, so a naive leave handler flickers. Count
	// enters/leaves and only drop out of the drag state when the counter zeroes.
	let dragDepth = 0;

	function onDragEnter(e) {
		e.preventDefault();
		dragDepth += 1;
		isDragging = true;
	}

	function onDragLeaveZone(e) {
		e.preventDefault();
		dragDepth = Math.max(0, dragDepth - 1);
		if (dragDepth === 0) isDragging = false;
	}

	function onDropZone(e) {
		dragDepth = 0;
		return handleDrop(e);
	}

	let stagedCount = $derived(files.length);
	let stagedBytes = $derived(files.reduce((a, f) => a + (f.file ? f.file.size : f.size), 0));

	function copyPassword() {
		navigator.clipboard.writeText(password);
		toast.success('Password copied', 'Without it the files cannot be decrypted.');
	}

	function copyLink() {
		if (!uploadSuccessUrl) return;
		navigator.clipboard.writeText(uploadSuccessUrl);
		toast.success('Link copied', 'Anyone with it can download the files.');
	}
</script>

<Seo
	title="Silocat: End-to-end encrypted file sharing & cloud storage"
	description="Zero-knowledge, end-to-end encrypted file sharing and cloud storage. Drop up to 20 GB, share a link that expires in seven days, and keep full control of your data."
	keywords="encrypted file sharing, secure cloud storage, zero knowledge storage, end-to-end encryption, anonymous file share, send large files, private file sharing"
	schema={softwareApplicationSchema()}
/>

<div class="page">
	<Navbar />

	<main class="main">
		<section class="hero">
			<h1>Big files. Zero knowledge.</h1>
			<p class="sub">
				End-to-end encrypted file transfer. No account needed. Drop up to 20&nbsp;GB and share a
				link that expires in seven days.
			</p>
		</section>

		<section class="zone-section">
			<!-- svelte-ignore a11y_no_static_element_interactions -->
			<div
				class="zone"
				class:dragging={zone === 'dragging'}
				ondragenter={onDragEnter}
				ondragover={handleDragOver}
				ondragleave={onDragLeaveZone}
				ondrop={onDropZone}
			>
				{#if zone === 'idle'}
					<div class="zone-idle">
						<Icon name="upload-lg" size={40} />
						<span class="zone-title">
							Drop files or
							<label for="file-upload" class="browse">browse</label>
						</span>
						<input type="file" id="file-upload" multiple onchange={handleFileSelect} hidden />
						<span class="zone-hint">
							encrypted before they leave ·
							<label for="folder-upload" class="folder-link">upload a folder</label>
						</span>
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
				{:else if zone === 'dragging'}
					<div class="zone-idle dragging-body">
						<Icon name="upload-lg" size={40} />
						<span class="zone-title">Let go to encrypt</span>
						<span class="zone-mono">drop to stage your files</span>
					</div>
				{:else if zone === 'staged'}
					<div class="staged">
						<div class="staged-list">
							{#each files as fileItem, i (i)}
								{@const f = fileItem.file || fileItem}
								<div class="staged-row" transition:fade={{ duration: 120 }}>
									<Icon name="file" size={16} class="row-glyph" />
									<div class="row-text">
										<span class="row-name">{f.name}</span>
										{#if fileItem.path}
											<span class="row-path">{fileItem.path}/</span>
										{/if}
									</div>
									<span class="row-size">{fmtSize(f.size)}</span>
									<button
										type="button"
										class="row-x"
										onclick={() => removeFile(i)}
										aria-label="Remove file"
									>
										<Icon name="close" size={13} />
									</button>
								</div>
							{/each}
						</div>

						<div class="staged-foot">
							<div class="pw-row">
								<button
									type="button"
									role="switch"
									aria-checked={encryptionEnabled}
									aria-label="Password protect"
									class="switch"
									class:on={encryptionEnabled}
									onclick={() => {
										encryptionEnabled = !encryptionEnabled;
										onEncryptionToggle();
									}}
								>
									<span class="knob"></span>
								</button>
								<div class="pw-text">
									<span class="pw-title">Password protect</span>
									<span class="pw-note">
										{encryptionEnabled
											? 'Only someone with the password can decrypt this.'
											: 'Uploads unencrypted.'}
									</span>
								</div>
							</div>

							{#if encryptionEnabled}
								<div class="pw-field" transition:fade={{ duration: 120 }}>
									<div class="pw-input">
										<input
											type={showPassword ? 'text' : 'password'}
											bind:value={password}
											placeholder="Password"
											autocomplete="new-password"
											spellcheck="false"
										/>
										<button
											type="button"
											aria-label={showPassword ? 'Hide password' : 'Show password'}
											onclick={() => (showPassword = !showPassword)}
										>
											<Icon name="eye" size={16} />
										</button>
										<button type="button" aria-label="Copy password" onclick={copyPassword}>
											<Icon name="copy" size={16} />
										</button>
									</div>
									<button
										type="button"
										class="generate"
										onclick={() => (password = generatePassword())}
									>
										Generate
									</button>
								</div>
							{/if}

							<div class="staged-actions">
								<button
									type="button"
									class="primary"
									disabled={encryptionEnabled && !password}
									onclick={startUpload}
								>
									Upload
								</button>
								<button type="button" class="quiet" onclick={() => (files = [])}>Clear all</button>
							</div>
						</div>
					</div>
				{:else}
					<div class="uploading">
						<div class="bar-block">
							<div class="bar-head">
								<span class="bar-label">Total</span>
								<span class="bar-pct">{Math.round(uploadStats.totalProgress)}%</span>
							</div>
							<div class="track lg">
								<div class="fill accent" style="width:{uploadStats.totalProgress}%"></div>
							</div>
						</div>
						<div class="bar-block">
							<div class="bar-head">
								<span class="bar-sub">{uploadStats.currentFileName || 'Processing'}</span>
								<span class="bar-pct sm">{Math.round(uploadStats.fileProgress)}%</span>
							</div>
							<div class="track">
								<div class="fill neutral" style="width:{uploadStats.fileProgress}%"></div>
							</div>
						</div>
						<div class="rate">
							<span>
								{uploadStats.speed
									? (uploadStats.speed / 1024 / 1024).toFixed(1) + ' MB/s'
									: 'starting'}
							</span>
							{#if uploadStats.eta}
								<span class="dot">·</span>
								<span>eta {Math.ceil(uploadStats.eta)}s</span>
							{/if}
						</div>
					</div>
				{/if}
			</div>

			<div class="steps">
				<span>drop it</span><span class="dot">·</span>
				<span>we encrypt it</span><span class="dot">·</span>
				<span>share the link</span>
			</div>
		</section>

		<section class="facts">
			<div class="fact">
				<h2>Zero-knowledge</h2>
				<p>
					Your files are encrypted in the browser, before a single byte moves. The server stores
					ciphertext and nothing else. We cannot read your files, hand them over, or recover them
					for you.
				</p>
			</div>
			<div class="fact">
				<h2>Anonymous</h2>
				<p>
					No account, no email, no card. A key kept in your browser lets you manage what you
					dropped. Lose the key and the upload is simply gone.
				</p>
			</div>
			<div class="fact">
				<h2>Fast</h2>
				<p>
					Encryption streams in chunks, so uploads start immediately and the share link works
					before the transfer finishes. Send it now, let the bytes catch up.
				</p>
			</div>
			<div class="fact">
				<h2>Whole folders</h2>
				<p>
					Drop a directory and the structure survives the trip. Up to 20&nbsp;GB per anonymous
					drop, kept for seven days. A free account gives you 10&nbsp;GB that stays until you
					delete it.
				</p>
			</div>
		</section>

		<section class="oss">
			<p>
				Silocat is open source under AGPL-3.0.
				<a href="https://github.com/clickswave/silocat" target="_blank" rel="noreferrer">
					Read the code
				</a>, or self-host it.
			</p>
		</section>

		<section class="cta">
			<h2>Ready to send something?</h2>
			<div class="cta-actions">
				<a href="/auth/signup" class="cta-primary">Create free account</a>
				<a href="/pricing" class="cta-ghost">See pricing</a>
			</div>
		</section>
	</main>

	<Footer />
</div>

<Modal
	open={showSuccessModal}
	title={isUploading ? 'Uploading' : 'Ready to share'}
	icon={isUploading ? 'upload' : 'check'}
	iconTone={isUploading ? 'neutral' : 'ok'}
	onclose={() => {
		if (!isUploading) resetUpload();
	}}
>
	<div class="ok-stack">
		<p class="ok-line">
			{#if isUploading}
				The link is live already. Send it now, the upload keeps running in the background.
			{:else}
				Anyone with this link can download {files.length > 1 ? 'these files' : 'this file'}.
			{/if}
		</p>

		{#if uploadSuccessUrl}
			<div class="link-row">
				<div class="link-box">
					<input type="text" readonly value={uploadSuccessUrl} onclick={(e) => e.target.select()} />
				</div>
				<button type="button" class="link-copy" onclick={copyLink}>Copy</button>
			</div>
		{/if}

		{#if isUploading}
			<div class="bar-block">
				<div class="bar-head">
					<span class="bar-sub">Uploading</span>
					<span class="bar-pct sm">{Math.round(uploadStats.totalProgress)}%</span>
				</div>
				<div class="track lg">
					<div class="fill accent" style="width:{uploadStats.totalProgress}%"></div>
				</div>
			</div>
		{/if}

		{#if encryptionEnabled && password}
			<div class="pw-save">
				<span class="pw-save-label">
					Decryption password, save it now. It is required to download.
				</span>
				<div class="pw-save-row">
					<span class="pw-save-value">{password}</span>
					<button type="button" aria-label="Copy password" onclick={copyPassword}>
						<Icon name="copy" size={15} />
					</button>
				</div>
			</div>
		{/if}
	</div>

	{#snippet footer()}
		<div class="ok-footer">
			{#if !data?.user}
				<div class="convert">
					<span>This link expires in 7 days. A free account keeps 10 GB forever.</span>
					<a href="/auth/signup">Create free account →</a>
				</div>
			{/if}
			<Button block disabled={isUploading} onclick={resetUpload}>
				{isUploading ? 'Uploading in background' : 'Upload another'}
			</Button>
		</div>
	{/snippet}
</Modal>

<style lang="scss">
	.page {
		min-height: 100vh;
		background: var(--bg);
		color: var(--ink);
		font-family: var(--font-sans);
		font-size: var(--fs-body);
		line-height: var(--lh-normal);
	}

	.main {
		max-width: var(--container);
		margin: 0 auto;
		padding-inline: var(--gutter);
	}

	/* ---- hero ---- */
	.hero {
		padding: clamp(3rem, 8vw, 6rem) 0 2.5rem;
		display: flex;
		flex-direction: column;
		align-items: center;
		text-align: center;
		gap: var(--space-4);

		h1 {
			margin: 0;
			font-size: var(--fs-display);
			font-weight: var(--fw-black);
			letter-spacing: var(--tracking-tight);
			line-height: var(--lh-tight);
		}
	}

	.sub {
		margin: 0;
		max-width: 52ch;
		font-size: var(--fs-lg);
		color: var(--ink-mute);
	}

	/* ---- drop zone ---- */
	.zone-section {
		padding-bottom: var(--space-4);
	}

	.zone {
		border: 1px dashed var(--edge-strong);
		border-radius: var(--radius-lg);
		background: var(--surface);
		transition:
			border-color var(--dur) var(--ease),
			background var(--dur) var(--ease);

		&.dragging {
			border-color: var(--accent);
			background: var(--accent-soft);
		}
	}

	.zone-idle {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 0.875rem;
		padding: clamp(2.5rem, 7vw, 4.5rem) 1.5rem;
		text-align: center;
		color: var(--ink-faint);

		&.dragging-body {
			color: var(--accent);
		}
	}

	.zone-title {
		font-size: 1.25rem;
		font-weight: var(--fw-medium);
		letter-spacing: var(--tracking-tight);
		color: var(--ink);
	}

	.browse {
		color: var(--accent);
		cursor: pointer;

		&:hover {
			color: var(--accent-hover);
		}
	}

	.zone-hint {
		font-size: var(--fs-sm);
		color: var(--ink-faint);
	}

	.folder-link {
		color: var(--ink-mute);
		cursor: pointer;
		text-decoration: underline;
		text-decoration-color: var(--edge-strong);
		text-underline-offset: 3px;

		&:hover {
			color: var(--ink);
		}
	}

	.zone-mono {
		font-family: var(--font-mono);
		font-size: var(--fs-sm);
		color: var(--ink-faint);
	}

	/* ---- staged ---- */
	.staged {
		display: flex;
		flex-direction: column;
	}

	.staged-list {
		max-height: 212px;
		overflow-y: auto;
	}

	.staged-row {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		padding: 0.75rem 1rem;
		border-bottom: 1px solid var(--edge);
		color: var(--ink-faint);
	}

	.row-text {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
	}

	.row-name {
		font-size: 0.875rem;
		color: var(--ink);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.row-path {
		font-family: var(--font-mono);
		font-size: var(--fs-xs);
		color: var(--ink-faint);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.row-size {
		flex: 0 0 auto;
		font-family: var(--font-mono);
		font-size: var(--fs-sm);
		color: var(--ink-faint);
	}

	.row-x {
		flex: 0 0 auto;
		width: 24px;
		height: 24px;
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
	}

	.staged-foot {
		display: flex;
		flex-direction: column;
		gap: 0.875rem;
		padding: 1rem;
	}

	.pw-row {
		display: flex;
		align-items: center;
		gap: var(--space-3);
	}

	.switch {
		flex: 0 0 auto;
		width: 34px;
		height: 20px;
		border: 0;
		border-radius: var(--radius-full);
		position: relative;
		background: var(--tint-softer);
		cursor: pointer;
		transition: background var(--dur-fast) var(--ease);

		&.on {
			background: var(--accent);
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
		&.on .knob {
			left: 16px;
		}
	}

	.pw-text {
		display: flex;
		flex-direction: column;
	}

	.pw-title {
		font-size: 0.875rem;
		font-weight: var(--fw-medium);
	}

	.pw-note {
		font-size: var(--fs-sm);
		color: var(--ink-faint);
	}

	.pw-field {
		display: flex;
		gap: 0.375rem;
	}

	.pw-input {
		flex: 1;
		display: flex;
		align-items: center;
		gap: var(--space-2);
		height: 38px;
		padding-inline: 0.75rem;
		border-radius: var(--radius-sm);
		background: var(--bg);
		border: 1px solid var(--edge);

		input {
			flex: 1;
			min-width: 0;
			border: 0;
			background: none;
			outline: none;
			font-family: var(--font-mono);
			font-size: 0.875rem;
			color: var(--ink);
		}

		button {
			border: 0;
			background: none;
			color: var(--ink-faint);
			cursor: pointer;
			display: grid;
			place-items: center;

			&:hover {
				color: var(--ink);
			}
		}
	}

	.generate {
		height: 38px;
		padding-inline: 0.875rem;
		border-radius: var(--radius-sm);
		border: 1px solid var(--edge);
		background: none;
		color: var(--ink);
		font: inherit;
		font-size: var(--fs-sm);
		font-weight: var(--fw-medium);
		cursor: pointer;
		transition:
			background var(--dur-fast) var(--ease),
			border-color var(--dur-fast) var(--ease);

		&:hover {
			background: var(--tint-soft);
			border-color: var(--edge-strong);
		}
	}

	.staged-actions {
		display: flex;
		align-items: center;
		gap: var(--space-3);
	}

	.primary {
		flex: 1;
		height: 42px;
		border: 0;
		border-radius: var(--radius-md);
		background: var(--accent);
		color: #fff;
		font: inherit;
		font-size: var(--fs-body);
		font-weight: var(--fw-medium);
		cursor: pointer;
		transition: background var(--dur-fast) var(--ease);

		&:hover:not(:disabled) {
			background: var(--accent-hover);
		}
		&:disabled {
			opacity: 0.5;
			cursor: not-allowed;
		}
	}

	.quiet {
		height: 42px;
		padding-inline: 1rem;
		border: 0;
		background: none;
		border-radius: var(--radius-md);
		font: inherit;
		font-size: 0.875rem;
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

	/* ---- uploading ---- */
	.uploading {
		display: flex;
		flex-direction: column;
		gap: 1.125rem;
		padding: 1.75rem;
	}

	.bar-block {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}

	.bar-head {
		display: flex;
		align-items: baseline;
		justify-content: space-between;
		gap: var(--space-4);
	}

	.bar-label {
		font-size: 0.875rem;
		font-weight: var(--fw-medium);
	}

	.bar-sub {
		font-size: var(--fs-sm);
		color: var(--ink-mute);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.bar-pct {
		font-family: var(--font-mono);
		font-size: 0.875rem;
		color: var(--ink-mute);

		&.sm {
			font-size: var(--fs-sm);
			color: var(--ink-faint);
		}
	}

	.track {
		height: 4px;
		border-radius: var(--radius-full);
		background: var(--tint-softer);
		overflow: hidden;

		&.lg {
			height: 6px;
		}
	}

	.fill {
		height: 100%;
		border-radius: var(--radius-full);
		transition: width var(--dur) var(--ease);

		&.accent {
			background: var(--accent);
		}
		&.neutral {
			background: var(--ink-faint);
		}
	}

	.rate {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		font-family: var(--font-mono);
		font-size: var(--fs-sm);
		color: var(--ink-faint);
	}

	.dot {
		opacity: 0.5;
	}

	.steps {
		display: flex;
		justify-content: center;
		gap: var(--space-2);
		padding-top: var(--space-4);
		font-family: var(--font-mono);
		font-size: var(--fs-xs);
		color: var(--ink-faint);
	}

	/* ---- facts ---- */
	.facts {
		max-width: var(--container-narrow);
		margin: 0 auto;
		padding: clamp(3.5rem, 9vw, 6rem) 0;
		display: grid;
		gap: 2.5rem;
	}

	.fact {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);

		h2 {
			margin: 0;
			font-size: 1.25rem;
			font-weight: var(--fw-semibold);
			letter-spacing: var(--tracking-tight);
		}
		p {
			margin: 0;
			color: var(--ink-mute);
		}
	}

	/* ---- open source ---- */
	.oss {
		max-width: var(--container-narrow);
		margin: 0 auto;
		padding-bottom: clamp(3.5rem, 9vw, 6rem);

		p {
			margin: 0;
			padding-block: 1.25rem;
			border-top: 1px solid var(--edge);
			border-bottom: 1px solid var(--edge);
			color: var(--ink-mute);
		}
	}

	/* ---- cta ---- */
	.cta {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: var(--space-5);
		padding-bottom: clamp(4rem, 10vw, 7rem);
		text-align: center;

		h2 {
			margin: 0;
			font-size: var(--fs-h2);
			font-weight: var(--fw-semibold);
			letter-spacing: var(--tracking-tight);
		}
	}

	.cta-actions {
		display: flex;
		flex-wrap: wrap;
		justify-content: center;
		gap: 0.625rem;
	}

	.cta-primary,
	.cta-ghost {
		display: flex;
		align-items: center;
		height: 46px;
		padding-inline: 1.375rem;
		border-radius: var(--radius-md);
		font-size: 1rem;
		font-weight: var(--fw-medium);
		text-decoration: none;
		transition:
			background var(--dur-fast) var(--ease),
			border-color var(--dur-fast) var(--ease);
	}

	.cta-primary {
		background: var(--accent);
		color: #fff;

		&:hover {
			background: var(--accent-hover);
			color: #fff;
		}
	}

	.cta-ghost {
		border: 1px solid var(--edge);
		color: var(--ink);

		&:hover {
			background: var(--tint-soft);
			border-color: var(--edge-strong);
			color: var(--ink);
		}
	}

	/* ---- success modal ---- */
	.ok-stack {
		display: flex;
		flex-direction: column;
		gap: 0.875rem;
	}

	.ok-line {
		margin: 0;
		font-size: 0.875rem;
		color: var(--ink-mute);
	}

	.link-row {
		display: flex;
		gap: 0.375rem;
	}

	.link-box {
		flex: 1;
		min-width: 0;
		display: flex;
		align-items: center;
		height: 38px;
		padding-inline: 0.75rem;
		border-radius: var(--radius-sm);
		background: var(--surface);
		border: 1px solid var(--edge);

		input {
			width: 100%;
			border: 0;
			background: none;
			outline: none;
			font-family: var(--font-mono);
			font-size: 0.875rem;
			color: var(--ink-mute);
		}
	}

	.link-copy {
		height: 38px;
		padding-inline: 0.875rem;
		border-radius: var(--radius-sm);
		border: 1px solid var(--edge);
		background: none;
		color: var(--ink);
		font: inherit;
		font-size: var(--fs-sm);
		font-weight: var(--fw-medium);
		cursor: pointer;
		transition: background var(--dur-fast) var(--ease);

		&:hover {
			background: var(--tint-soft);
		}
	}

	.pw-save {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		padding: 0.75rem;
		border-radius: 8px;
		background: var(--warn-soft);
		border: 1px solid var(--edge);
	}

	.pw-save-label {
		font-size: var(--fs-xs);
		color: var(--ink-mute);
	}

	.pw-save-row {
		display: flex;
		align-items: center;
		gap: var(--space-2);

		button {
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
				background: var(--tint-softer);
				color: var(--ink);
			}
		}
	}

	.pw-save-value {
		flex: 1;
		font-family: var(--font-mono);
		font-size: 0.9375rem;
		font-weight: var(--fw-medium);
		word-break: break-all;
	}

	.ok-footer {
		display: flex;
		flex-direction: column;
		gap: 0.875rem;
		width: 100%;
	}

	/* The one place the product asks for the account: right after the value
	   landed, while the 7-day expiry is the thing on the sender's mind. */
	.convert {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-4);
		padding: 0.75rem;
		margin: -0.25rem -0.25rem 0;
		border-radius: 8px;
		background: var(--tint-soft);
		font-size: var(--fs-sm);
		color: var(--ink-mute);

		a {
			flex: 0 0 auto;
			font-weight: var(--fw-medium);
			text-decoration: none;
		}
	}

	@media (max-width: 640px) {
		.convert {
			flex-direction: column;
			align-items: flex-start;
			gap: var(--space-2);
		}
	}
</style>
