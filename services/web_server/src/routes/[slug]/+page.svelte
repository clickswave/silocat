<script>
	import { page } from '$app/stores';
	import { onMount } from 'svelte';
	import axios from 'axios';
	import sodium from 'libsodium-wrappers-sumo';
	import { decryptChunk, deriveKeyFromPassword } from '$lib/chacha.js';
	import { toast } from 'svelte-sonner';
	import Icon from '@iconify/svelte';
	import { fade, scale } from 'svelte/transition';
	import Navbar from '$lib/components/Navbar.svelte';
	import Footer from '$lib/components/Footer.svelte';
	import { shadowKey } from '$lib/stores/shadow.js';

	const CHUNK_SIZE = 100 * 1024 * 1024; // 100MB

	let fileId = $page.params.slug;
	let fileMeta = $state(null);
	/* ... state ... */

	// Delete State
	let showDeleteModal = $state(false);
	let deleteKeyInput = $state('');
	let isDeleting = $state(false);

	function handleDeleteClick() {
		const user = $page.data.user;
		if (user && user.api_key) {
			deleteKeyInput = user.api_key;
		} else if ($shadowKey) {
			deleteKeyInput = $shadowKey;
		}
		showDeleteModal = true;
	}

	async function performDelete() {
		if (!deleteKeyInput) {
			toast.error('API Key is required');
			return;
		}
		isDeleting = true;
		try {
			try {
				const endpoint = isFolder ? '/api/v1/shadow/folder/delete' : '/api/v1/shadow/file/delete';
				const body = isFolder
					? { folder_id: folderMeta.id, api_key: deleteKeyInput }
					: { file_id: fileMeta.id, api_key: deleteKeyInput };

				await axios.post(endpoint, body);

				toast.success(`${isFolder ? 'Folder' : 'File'} deleted successfully`);
				showDeleteModal = false;
				setTimeout(() => (window.location.href = '/'), 1500);
			} catch (e) {
				console.error(e);
				let msg =
					(Array.isArray(e.response?.data?.errors) && e.response.data.errors.length > 0
						? e.response.data.errors[0]
						: e.response?.data?.message) || `Failed to delete ${isFolder ? 'folder' : 'file'}`;
				if (e.response?.status === 403) msg = 'Incorrect API Key. Permission denied.';
				toast.error(msg);
			}
		} finally {
			isDeleting = false;
		}
	}
	let isFolder = $state(false);
	let folderMeta = $state(null);
	let folderFiles = $state([]);

	// Password / Encryption State
	let password = $state('');
	let key = $state(null);

	let isDownloading = $state(false);
	let progress = $state(0);
	let downloadUrl = $state(null);
	let error = $state(null);

	let pollingInterval;

	onMount(async () => {
		await fetchMetadata();
		// Start polling if it's a folder OR if single file is still uploading
		if (isFolder || (fileMeta && uploadProgress < 100)) {
			pollingInterval = setInterval(async () => {
				await fetchMetadata(true);
				// Stop polling if single file upload is complete
				if (!isFolder && uploadProgress >= 100) {
					clearInterval(pollingInterval);
				}
			}, 3000);
		}
		return () => {
			if (pollingInterval) clearInterval(pollingInterval);
		};
	});

	async function fetchMetadata(isPolling = false) {
		try {
			// Single Unified Call
			const res = await axios.post('/api/v1/shadow/resource/fetch', { id: fileId });

			if (res.data.data) {
				const { resource_type, file, folder, files, folders } = res.data.data;

				if (resource_type === 'file') {
					fileMeta = file;
					isFolder = false;
					if (fileMeta.total_chunks) {
						uploadProgress = (fileMeta.uploaded_chunks / fileMeta.total_chunks) * 100;
					}
				} else if (resource_type === 'folder') {
					isFolder = true;
					folderMeta = folder;
					// Preserve existing file status/progress if polling?
					// For now simply update the list. Svelte keying usually handles diffs.
					folderFiles = files || [];
					folderFiles = files || [];
					// folders (subfolders) ignored for now per current UI logic, or can be added
				} else {
					if (!isPolling) error = 'Unknown resource type.';
				}
			} else {
				error = 'Resource not found or expired.';
			}
		} catch (e) {
			console.error(e);
			error = 'Failed to load resource.';
			if (e.response?.status === 404) {
				error = 'Resource not found or expired.';
			}
		}
	}

	let currentDownloadId = $state(null);
	let currentDownloadProgress = $state(0);

	async function startDownloadSingle(file) {
		if (isDownloading) return;
		currentDownloadId = file.id;
		fileMeta = file; // Set context for download logic (or refactor to pass file)
		// But startDownload uses fileId global, we need to fix that.
		// Let's refactor startDownload to take an ID.
		await startDownload(file);
		currentDownloadId = null;
	}

	let isWaiting = $state(false);
	let uploadProgress = $state(0);

	async function downloadFileBytes(fileTarget, onProgress) {
		const downloadedChunks = [];
		let downloadedBytes = 0;
		let processedChunkCount = 0;
		const totalExpectedChunks = Math.ceil(fileTarget.size / CHUNK_SIZE);

		let key = null; // Re-derive key per file
		let isLocalWaiting = false;

		await sodium.ready;

		// Key derivation (lazy or upfront)
		if (fileTarget.encrypted) {
			if (!password) throw new Error('Password required to decrypt this file.');
		}

		while (downloadedBytes < fileTarget.size) {
			// Check for cancellation signal if needed, but for now rely on caller handling?
			// Actually we need to respect isDownloading global... but that's shared.
			// Ideally passed in signal. For now, check global.
			if (!isDownloading) throw new Error('Download cancelled');

			const chunksRes = await axios.post('/api/v1/shadow/file/fetch-chunks', {
				file_id: fileTarget.id
			});
			const chunks = chunksRes.data.data.chunks || [];

			if (chunks.length > processedChunkCount) {
				isLocalWaiting = false;

				// Derivation check (lazy init)
				if (fileTarget.encrypted && !key && chunks.length > 0) {
					const firstChunk = chunks[0];
					if (firstChunk.salt) {
						const saltBytes = Uint8Array.from(atob(firstChunk.salt), (c) => c.charCodeAt(0));
						key = await deriveKeyFromPassword(password, saltBytes);
					}
				}

				for (let i = processedChunkCount; i < chunks.length; i++) {
					if (!isDownloading) break;
					const chunk = chunks[i];

					let chunkData;
					try {
						const res = await axios.get(chunk.presigned_url, {
							responseType: 'arraybuffer',
							onDownloadProgress: (progressEvent) => {
								if (onProgress) {
									const currentChunkLoaded = progressEvent.loaded;
									const totalLoaded = downloadedBytes + currentChunkLoaded;
									const pct = (totalLoaded / fileTarget.size) * 100;
									onProgress(pct);
								}
							}
						});
						chunkData = res.data;
					} catch (e) {
						console.warn('Chunk not ready or failed fetch', e);
						break;
					}

					let dataBytes = new Uint8Array(chunkData);
					let finalBytes = dataBytes;

					if (fileTarget.encrypted) {
						if (!key) throw new Error('Encryption key not initialized (missing salt?)');
						if (!chunk.nonce) throw new Error(`Chunk ${i} missing nonce`);
						const nonceBytes = Uint8Array.from(atob(chunk.nonce), (c) => c.charCodeAt(0));
						try {
							finalBytes = await decryptChunk(dataBytes, key, nonceBytes);
						} catch (err) {
							throw new Error('Decryption failed. Incorrect password?');
						}
					}

					downloadedChunks.push(finalBytes);
					downloadedBytes += chunk.size;
					processedChunkCount++;

					if (onProgress) {
						onProgress((downloadedBytes / fileTarget.size) * 100);
					}
				}
			} else {
				// No new chunks yet.
				isLocalWaiting = true;
				// Update generic waiting UI if needed?
				// We can't update global uploadProgress easily here without context.
				await new Promise((r) => setTimeout(r, 3000)); // Wait 3s
			}
		}

		return new Blob(downloadedChunks, { type: fileTarget.mime });
	}

	async function startDownload(targetFile = null) {
		// Use targetFile if provided, otherwise fail or use fileMeta
		const fileTarget = targetFile || fileMeta;
		if (!fileTarget) return;

		isDownloading = true;
		progress = 0;
		currentDownloadProgress = 0;
		uploadProgress = 0;
		downloadUrl = null;
		isWaiting = false;

		try {
			const blob = await downloadFileBytes(fileTarget, (pct) => {
				progress = pct;
				currentDownloadProgress = pct;
				// Logic to update uploadProgress/isWaiting based on polling is encapsulated in helper?
				// Not fully. downloadFileBytes encapsulates the LOOP.
				// If we want "uploadProgress" UI (remote progress), we need to pass back data.
				// For now, simple progress bar is enough.
			});

			if (!isDownloading) return; // Cancelled

			downloadUrl = URL.createObjectURL(blob);

			const a = document.createElement('a');
			a.href = downloadUrl;
			a.download = fileTarget.name;
			document.body.appendChild(a);
			a.click();
			document.body.removeChild(a);

			toast.success('Download complete!');
		} catch (e) {
			console.error(e);
			toast.error('Download failed: ' + e.message);
		} finally {
			// Only clear if not in zip mode? startDownload is single.
			isDownloading = false;
		}
	}
	import JSZip from 'jszip';

	function generateRandomString(length) {
		const chars = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';
		let result = '';
		for (let i = 0; i < length; i++) {
			result += chars.charAt(Math.floor(Math.random() * chars.length));
		}
		return result;
	}

	async function startDownloadZip() {
		if (isDownloading) return;
		isDownloading = true;
		progress = 0;

		try {
			await sodium.ready;
			const zip = new JSZip();
			let processedFiles = 0;

			// Check password once if any file is encrypted (assuming shared password)
			const hasEncrypted = folderFiles.some((f) => f.encrypted);
			if (hasEncrypted && !password) {
				toast.error('Password required to decrypt files.');
				isDownloading = false;
				return;
			}

			// We process files sequentially or parallel?
			// Parallel would be faster but harder to track progress. Sequentially is safer for now.
			processedFiles = 0;

			for (const file of folderFiles) {
				try {
					// Reuse the polling/download logic
					const fileBlob = await downloadFileBytes(file, (pct) => {
						// Update total progress based on current file progress
						// total_progress = (processed + current_pct/100) / total * 100
						const currentFileContribution = pct / 100;
						progress = ((processedFiles + currentFileContribution) / folderFiles.length) * 100;
					});
					zip.file(file.name, fileBlob);
				} catch (e) {
					console.error(`Failed to download ${file.name} for zip`, e);
					toast.error(`Could not download ${file.name}: ${e.message}`);
					// Continue or abort? Abort seems safer to avoid partial zip.
					throw e;
				}

				processedFiles++;
				progress = (processedFiles / folderFiles.length) * 100;
			}

			const zipContent = await zip.generateAsync({ type: 'blob' });
			const zipName = (folderMeta ? folderMeta.name : 'download') + '.zip';
			const url = URL.createObjectURL(zipContent);

			const a = document.createElement('a');
			a.href = url;
			a.download = zipName;
			document.body.appendChild(a);
			a.click();
			document.body.removeChild(a);
			URL.revokeObjectURL(url);

			toast.success('All files downloaded!');
		} catch (e) {
			console.error(e);
			toast.error('Failed to create zip: ' + e.message);
		} finally {
			isDownloading = false;
			progress = 0;
		}
	}
</script>

<svelte:head>
	<title>
		{fileMeta ? fileMeta.name : folderMeta ? folderMeta.name : 'Secure Download'} - SiloCat
	</title>
	<meta
		name="description"
		content="Download {fileMeta
			? fileMeta.name
			: folderMeta
				? folderMeta.name
				: 'files'} securely from SiloCat. Kitty powered E2E encrypted anonymous file-sharing."
	/>
	<meta property="og:title" content="Secure Download - SiloCat" />
	<meta
		property="og:description"
		content="Kitty powered E2E encrypted anonymous file-sharing and cloud storage platform with parallel downloads."
	/>
</svelte:head>

<div class="landing-page">
	<Navbar />

	<main class="hero">
		<div class="hero-content">
			<h1>Ready to <span class="text-gradient">Download.</span></h1>
			<p class="subtitle">Secure, fast, and anonymous delivery.</p>

			<div class="download-card">
				{#if error}
					<div class="state-container error">
						<Icon icon="mdi:alert-circle" class="state-icon text-red-500" />
						<h3>Access Denied</h3>
						<p>{error}</p>
					</div>
				{:else if isFolder}
					<div class="file-details" transition:fade>
						<div class="icon-circle">
							<Icon icon="ri:folder-shared-line" width="48" />
						</div>

						<h2 class="filename" title={folderMeta.name}>
							{folderMeta.uploaded_as_files ? 'Shared Files' : folderMeta.name}
						</h2>
						<div class="action-header">
							<p class="filesize">{folderFiles.length} item{folderFiles.length !== 1 ? 's' : ''}</p>
							<div style="display: flex; gap: 0.5rem;">
								<button
									class="zip-btn"
									onclick={() => startDownloadZip()}
									disabled={isDownloading || (folderFiles.some((f) => f.encrypted) && !password)}
								>
									{#if isDownloading}
										<div
											style="display: flex; flex-direction: column; align-items: center; width: 100%;"
										>
											<div
												style="width: 100%; display: flex; align-items: center; justify-content: center; gap: 8px;"
											>
												<Icon icon="line-md:loading-loop" />
												<span>Processing... {Math.round(progress)}%</span>
											</div>
											<div
												style="width: 100%; height: 4px; background: rgba(255,255,255,0.2); border-radius: 2px; margin-top: 4px; overflow: hidden;"
											>
												<div
													style="width: {progress}%; height: 100%; background: white; transition: width 0.2s ease;"
												></div>
											</div>
										</div>
									{:else}
										<Icon icon="ri:file-zip-line" />
										Download Zip
									{/if}
								</button>
								<button class="delete-btn" onclick={handleDeleteClick} title="Delete Folder">
									<Icon icon="ri:delete-bin-line" width="20" />
								</button>
							</div>
						</div>

						<!-- Shared Password Input for Folder -->
						{#if folderFiles.some((f) => f.encrypted)}
							<div class="password-section">
								<div class="input-label">
									<Icon icon="ri:lock-password-line" />
									<span>Enter Password (if needed)</span>
								</div>
								<input
									type="password"
									bind:value={password}
									placeholder="Unlock password..."
									class="password-input"
								/>
							</div>
						{/if}

						<div class="file-list">
							{#each folderFiles as file}
								<div class="file-item">
									<div class="file-info">
										<Icon icon="ri:file-line" class="file-icon" />
										<div class="file-text">
											<span class="name">{file.name}</span>
											<span class="size">{(file.size / 1024 / 1024).toFixed(2)} MB</span>
										</div>
									</div>
									<div
										style="display: flex; flex-direction: column; align-items: flex-end; gap: 4px; min-width: 120px;"
									>
										{#if isDownloading && currentDownloadId === file.id && currentDownloadProgress > 0}
											<div style="width: 100%; display: flex; align-items: center; gap: 6px;">
												<div
													style="flex-grow: 1; height: 4px; background: rgba(255,255,255,0.1); border-radius: 2px; overflow: hidden;"
												>
													<div
														style="width: {currentDownloadProgress}%; height: 100%; background: #4facfe; transition: width 0.1s linear;"
													></div>
												</div>
												<span
													style="font-size: 0.75rem; color: rgba(255,255,255,0.8); font-variant-numeric: tabular-nums; width: 32px; text-align: right;"
												>
													{Math.round(currentDownloadProgress)}%
												</span>
											</div>
										{:else if file.total_chunks && file.uploaded_chunks < file.total_chunks}
											<div
												style="display: flex; align-items: center; gap: 4px; color: #ffb703; font-size: 0.75rem;"
											>
												<Icon icon="line-md:uploading-loop" />
												<span
													>{Math.round((file.uploaded_chunks / file.total_chunks) * 100)}% Uploaded</span
												>
											</div>
										{/if}

										<button
											class="mini-download-btn"
											onclick={() => startDownloadSingle(file)}
											disabled={(file.encrypted && !password) ||
												(isDownloading && currentDownloadId === file.id)}
											title="Download File"
										>
											{#if isDownloading && currentDownloadId === file.id}
												<Icon icon="line-md:loading-loop" />
											{:else}
												<Icon icon="ri:download-line" />
											{/if}
										</button>
									</div>
								</div>
							{/each}
						</div>
						<div class="metadata-section">
							<h3>Folder Details</h3>
							<div class="meta-grid">
								<div class="meta-item">
									<span class="label">Type</span>
									<span class="value"
										>{folderMeta.uploaded_as_files ? 'File Collection' : 'Folder'}</span
									>
								</div>
								<div class="meta-item">
									<span class="label">Items</span>
									<span class="value">{folderFiles.length} files</span>
								</div>
								<div class="meta-item">
									<span class="label">Created On</span>
									<span class="value">{new Date(folderMeta.created_on).toLocaleDateString()}</span>
								</div>
								<div class="meta-item">
									<span class="label">Total Size</span>
									<span class="value"
										>{(folderFiles.reduce((acc, f) => acc + f.size, 0) / 1024 / 1024).toFixed(2)} MB</span
									>
								</div>
							</div>
						</div>
					</div>
				{:else if fileMeta}
					<div class="file-details" transition:fade>
						<div class="icon-circle">
							<Icon icon="ri:file-download-line" width="48" />
						</div>

						<h2 class="filename" title={fileMeta.name}>{fileMeta.name}</h2>
						<p class="filesize">{(fileMeta.size / 1024 / 1024).toFixed(2)} MB</p>

						{#if fileMeta.encrypted}
							<div class="password-section">
								<div class="input-label">
									<Icon icon="ri:lock-password-line" />
									<span>Enter Password to Decrypt</span>
								</div>
								<input
									type="password"
									bind:value={password}
									placeholder="Unlock password..."
									class="password-input"
								/>
							</div>
						{:else}
							<div class="info-badge">
								<Icon icon="ri:shield-check-line" />
								<span>File is not password protected.</span>
							</div>
						{/if}

						<div class="action-area">
							{#if isDownloading}
								<div class="progress-container">
									<div class="progress-bar-bg">
										<!-- Remote Upload Progress (Lighter/ Different Color) -->
										<div
											class="progress-bar-fill remote"
											style="width: {uploadProgress}%; background-color: rgba(255,255,255,0.2); position: absolute; top:0; left:0; height: 100%;"
										></div>
										<!-- Local Download Progress (Primary Color) -->
										<div
											class="progress-bar-fill local"
											style="width: {progress}%; position: absolute; top:0; left:0; height: 100%;"
										></div>
									</div>
									<div
										class="progress-info-row"
										style="display: flex; justify-content: space-between; margin-top: 8px;"
									>
										<span class="progress-text">
											{fileMeta.encrypted ? 'Decrypting' : 'Downloading'}... {Math.round(progress)}%
										</span>
										{#if isWaiting}
											<span
												class="waiting-text"
												style="color: var(--warning-color, #ffaa00); display: flex; align-items: center; gap: 4px;"
											>
												<Icon icon="svg-spinners:dots-12" /> Waiting for upload... {Math.round(
													uploadProgress
												)}%
											</span>
										{:else if uploadProgress < 100}
											<span class="remote-text" style="opacity: 0.7;">
												Uploaded: {Math.round(uploadProgress)}%
											</span>
										{/if}
									</div>
								</div>
							{:else}
								<button
									class="download-btn"
									onclick={() => startDownload()}
									disabled={fileMeta.encrypted && !password}
								>
									{#if fileMeta.encrypted}
										<Icon icon="ri:lock-unlock-line" width="24" />
										Decrypt & Download
									{:else}
										<Icon icon="ri:download-cloud-2-line" width="24" />
										Download File
									{/if}
								</button>
								<button class="delete-btn" onclick={handleDeleteClick} title="Delete File">
									<Icon icon="ri:delete-bin-line" width="24" />
								</button>
							{/if}
						</div>

						<div class="metadata-section">
							<h3>File Details</h3>
							<div class="meta-grid">
								<div class="meta-item">
									<span class="label">Type</span>
									<span class="value">File</span>
								</div>
								<div class="meta-item">
									<span class="label">Size</span>
									<span class="value">{(fileMeta.size / 1024 / 1024).toFixed(2)} MB</span>
								</div>
								<div class="meta-item">
									<span class="label">Date Uploaded</span>
									<span class="value">{new Date(fileMeta.created_on).toLocaleDateString()}</span>
								</div>
								<div class="meta-item">
									<span class="label">MIME Type</span>
									<span class="value">{fileMeta.mime}</span>
								</div>
								<div class="meta-item full-width">
									<span class="label">SHA256 Checksum</span>
									<span class="value mono">{fileMeta.sha256_checksum}</span>
								</div>
							</div>
						</div>
					</div>
				{:else}
					<div class="state-container loading">
						<Icon icon="svg-spinners:ring-resize" class="state-icon" />
						<p>Retrieving secure metadata...</p>
					</div>
				{/if}
			</div>
		</div>
	</main>

	<Footer />

	<div class="bg-effects">
		<div class="glow-spot top"></div>
		<div class="glow-spot bottom"></div>
	</div>
</div>

{#if showDeleteModal}
	<div class="modal-backdrop" transition:fade onclick={() => (showDeleteModal = false)}>
		<div class="modal-content" transition:scale onclick={(e) => e.stopPropagation()}>
			<div class="modal-header">
				<Icon icon="ri:delete-bin-fill" width="32" class="modal-icon error" />
				<h2>Delete {isFolder ? 'Folder' : 'File'}</h2>
			</div>

			<div class="modal-body">
				<p>
					Are you sure you want to delete this {isFolder ? 'folder' : 'file'}? This action cannot be
					undone.
				</p>

				<div class="input-group">
					<label for="delKey">Owner API Key</label>
					<div class="input-wrapper">
						<input
							type="text"
							id="delKey"
							bind:value={deleteKeyInput}
							placeholder="Enter API Key to verify ownership"
							class:has-value={!!deleteKeyInput}
						/>
						{#if $page.data.user && deleteKeyInput === $page.data.user.api_key}
							<span
								class="badge"
								style="background: rgba(255, 70, 85, 0.2); color: #ff4655; border-color: rgba(255, 70, 85, 0.3);"
								>Account Key</span
							>
						{:else if $shadowKey && deleteKeyInput === $shadowKey}
							<span class="badge">Browser Key</span>
						{/if}
					</div>
					<p class="hint">
						Required to verify ownership. If you lost the key, the {isFolder ? 'folder' : 'file'} cannot
						be deleted.
					</p>
				</div>
			</div>

			<div class="modal-actions">
				<button class="cancel-btn" onclick={() => (showDeleteModal = false)}>Cancel</button>
				<button
					class="confirm-delete-btn"
					onclick={performDelete}
					disabled={isDeleting || !deleteKeyInput}
				>
					{#if isDeleting}
						<Icon icon="svg-spinners:ring-resize" /> Deleting...
					{:else}
						<Icon icon="ri:delete-bin-line" /> Delete
					{/if}
				</button>
			</div>
		</div>
	</div>
{/if}

<style lang="scss">
	.landing-page {
		min-height: 100vh;
		position: relative;
		overflow: hidden;
		display: flex;
		flex-direction: column;
	}

	.hero {
		flex: 1;
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 10;
		padding: var(--space-8) var(--gutter);
		.hero-content {
			text-align: center;
			max-width: 800px;
			width: 100%;
			display: flex;
			flex-direction: column;
			align-items: center;
			gap: var(--space-5);
			h1 {
				font-size: var(--fs-display);
				font-weight: var(--fw-black);
				margin: 0;
			}
			.subtitle {
				font-size: var(--fs-lg);
				color: var(--text-secondary);
				margin-bottom: var(--space-5);
			}
		}
	}

	.download-card {
		background: var(--bg-card);
		border: 1px solid var(--border-default);
		border-radius: var(--radius-lg);
		padding: clamp(1.5rem, 5vw, 3rem);
		width: 100%;
		max-width: 500px;
		box-shadow: var(--shadow-lg);
		min-height: 300px;
		display: flex;
		align-items: center;
		justify-content: center;

		.state-container {
			display: flex;
			flex-direction: column;
			align-items: center;
			gap: var(--space-4);
			.state-icon {
				font-size: 3rem;
				color: var(--primary);
			}
			h3 {
				font-size: var(--fs-h3);
				font-weight: var(--fw-semibold);
				margin: 0;
			}
			p {
				color: var(--text-secondary);
			}
			&.loading .state-icon {
				color: var(--text-secondary);
			}
		}

		.file-details {
			width: 100%;
			display: flex;
			flex-direction: column;
			align-items: center;
			gap: var(--space-2);
			.icon-circle {
				width: 80px;
				height: 80px;
				background: rgba(255, 70, 85, 0.1);
				border-radius: 50%;
				display: flex;
				align-items: center;
				justify-content: center;
				color: var(--primary);
				box-shadow: var(--shadow-glow);
				margin-bottom: var(--space-4);
			}
			.filename {
				font-size: var(--fs-h3);
				font-weight: var(--fw-semibold);
				margin: 0;
				word-break: break-all;
			}
			.filesize {
				color: var(--text-muted);
				font-size: var(--fs-body);
				margin-bottom: var(--space-6);
			}

			.action-header {
				display: flex;
				justify-content: space-between;
				align-items: center;
				width: 100%;
				margin-bottom: var(--space-5);

				.filesize {
					color: var(--text-muted);
					font-size: var(--fs-body);
					margin: 0;
				}

				.zip-btn {
					background: var(--tint-soft);
					border: 1px solid var(--border-default);
					color: var(--text-primary);
					padding: 0.5rem 1rem;
					border-radius: var(--radius-sm);
					font-size: var(--fs-sm);
					font-weight: var(--fw-medium);
					font-family: inherit;
					cursor: pointer;
					display: flex;
					align-items: center;
					gap: var(--space-2);
					transition: background var(--dur) var(--ease), border-color var(--dur) var(--ease);

					&:hover:not(:disabled) {
						background: var(--tint-softer);
						border-color: var(--border-strong);
					}

					&:disabled {
						opacity: 0.5;
						cursor: not-allowed;
					}
				}
			}

			.file-list {
				width: 100%;
				display: flex;
				flex-direction: column;
				gap: var(--space-3);
				margin-bottom: var(--space-6);
				max-height: 400px;
				overflow-y: auto;
				padding-right: var(--space-2);

				.file-item {
					display: flex;
					justify-content: space-between;
					align-items: center;
					padding: var(--space-4);
					background: var(--tint-soft);
					border: 1px solid var(--hairline);
					border-radius: var(--radius-sm);
					transition: background var(--dur) var(--ease), border-color var(--dur) var(--ease);

					&:hover {
						background: var(--tint-softer);
						border-color: var(--border-default);
					}

					.file-info {
						display: flex;
						align-items: center;
						gap: var(--space-4);
						overflow: hidden;

						.file-icon {
							font-size: 1.5rem;
							color: var(--text-secondary);
							flex-shrink: 0;
						}

						.file-text {
							display: flex;
							flex-direction: column;
							overflow: hidden;

							.name {
								font-weight: var(--fw-medium);
								white-space: nowrap;
								overflow: hidden;
								text-overflow: ellipsis;
								max-width: 200px;
							}

							.size {
								font-size: var(--fs-sm);
								color: var(--text-muted);
							}
						}
					}

					.mini-download-btn {
						background: var(--tint-softer);
						border: 1px solid var(--border-default);
						color: var(--text-primary);
						width: 36px;
						height: 36px;
						border-radius: var(--radius-sm);
						display: flex;
						align-items: center;
						justify-content: center;
						cursor: pointer;
						transition: background var(--dur) var(--ease);

						&:hover:not(:disabled) {
							background: var(--bg-card-hover);
						}

						&:disabled {
							opacity: 0.5;
							cursor: not-allowed;
						}
					}
				}
			}

			.password-section {
				width: 100%;
				background: var(--tint-soft);
				padding: var(--space-5);
				border-radius: var(--radius-sm);
				display: flex;
				flex-direction: column;
				gap: var(--space-3);
				margin-bottom: var(--space-5);
				border: 1px solid var(--hairline);

				.input-label {
					display: flex;
					align-items: center;
					gap: var(--space-2);
					color: var(--text-secondary);
					font-size: var(--fs-sm);
					font-weight: var(--fw-medium);
				}
				.password-input {
					width: 100%;
					background: var(--bg-input);
					border: 1px solid var(--border-default);
					color: var(--text-primary);
					padding: 0.75rem 0.95rem;
					border-radius: var(--radius-sm);
					font-size: var(--fs-body);
					font-family: var(--font-mono);
					outline: none;
					transition: border-color var(--dur) var(--ease), box-shadow var(--dur) var(--ease);
					box-sizing: border-box;
					&:focus {
						border-color: var(--primary);
						box-shadow: 0 0 0 3px var(--primary-glow);
					}
				}
			}

			.info-badge {
				background: rgba(61, 220, 151, 0.12);
				color: var(--success);
				padding: 0.6rem 1rem;
				border-radius: var(--radius-pill);
				font-size: var(--fs-sm);
				display: flex;
				align-items: center;
				gap: var(--space-2);
				margin-bottom: var(--space-6);
			}

			.action-area {
				width: 100%;
				.download-btn {
					width: 100%;
					background: var(--accent-gradient);
					color: #fff;
					border: none;
					padding: 0.95rem;
					border-radius: var(--radius-pill);
					font-weight: var(--fw-semibold);
					font-size: var(--fs-lg);
					font-family: inherit;
					cursor: pointer;
					display: flex;
					align-items: center;
					justify-content: center;
					gap: var(--space-3);
					box-shadow: 0 6px 20px -6px var(--primary-glow);
					transition: filter var(--dur) var(--ease), transform var(--dur) var(--ease);
					&:hover {
						filter: brightness(1.06);
						transform: translateY(-1px);
					}
					&:disabled {
						opacity: 0.5;
						cursor: not-allowed;
						transform: none;
						background: var(--border-strong);
						box-shadow: none;
					}
				}

				.progress-container {
					width: 100%;
					.progress-bar-bg {
						height: 8px;
						background: var(--tint-softer);
						border-radius: var(--radius-pill);
						overflow: hidden;
						margin-bottom: var(--space-2);
						position: relative;
						.progress-bar-fill {
							height: 100%;
							background: var(--accent-gradient);
							transition: width 0.3s ease;
						}
					}
					.progress-text {
						font-size: var(--fs-sm);
						color: var(--text-secondary);
					}
				}
			}
		}
	}

	.bg-effects {
		position: fixed;
		inset: 0;
		width: 100%;
		height: 100%;
		pointer-events: none;
		z-index: 0;
		overflow: hidden;
		.glow-spot {
			position: absolute;
			width: 600px;
			height: 600px;
			filter: blur(110px);
			border-radius: 50%;
			&.top {
				top: -22%;
				left: 12%;
				background: radial-gradient(circle, rgba(255, 70, 85, 0.16) 0%, transparent 70%);
			}
			&.bottom {
				bottom: -25%;
				right: 10%;
				width: 800px;
				height: 800px;
				background: radial-gradient(circle, rgba(74, 163, 226, 0.1) 0%, transparent 70%);
			}
		}
	}
	.metadata-section {
		width: 100%;
		margin-top: var(--space-6);
		padding-top: var(--space-6);
		border-top: 1px solid var(--hairline);

		h3 {
			font-size: var(--fs-lg);
			font-weight: var(--fw-semibold);
			margin-bottom: var(--space-4);
			color: var(--text-primary);
		}

		.meta-grid {
			display: grid;
			grid-template-columns: repeat(2, 1fr);
			gap: var(--space-4);

			.meta-item {
				display: flex;
				flex-direction: column;
				gap: var(--space-1);

				&.full-width {
					grid-column: span 2;
				}

				.label {
					font-size: var(--fs-xs);
					color: var(--text-muted);
					text-transform: uppercase;
					letter-spacing: 0.05em;
				}

				.value {
					font-size: var(--fs-sm);
					color: var(--text-primary);
					word-break: break-all;

					&.mono {
						font-family: var(--font-mono);
						font-size: var(--fs-sm);
						color: var(--text-secondary);
					}
				}
			}
		}
	}
	.action-area {
		display: flex;
		gap: var(--space-4);
		width: 100%;

		.download-btn {
			flex: 1;
		}

		.delete-btn {
			background: rgba(255, 70, 85, 0.1);
			border: 1px solid rgba(255, 70, 85, 0.25);
			color: var(--danger);
			padding: 0 var(--space-5);
			border-radius: var(--radius-pill);
			cursor: pointer;
			transition: background var(--dur) var(--ease), border-color var(--dur) var(--ease);
			display: flex;
			align-items: center;
			justify-content: center;

			&:hover {
				background: rgba(255, 70, 85, 0.18);
				border-color: rgba(255, 70, 85, 0.4);
			}
		}
	}

	.modal-backdrop {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.65);
		backdrop-filter: blur(8px);
		z-index: 100;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: var(--gutter);
	}
	.modal-content {
		background: var(--bg-elevated);
		border: 1px solid var(--border-default);
		border-radius: var(--radius-lg);
		padding: var(--space-6);
		width: 100%;
		max-width: 420px;
		display: flex;
		flex-direction: column;
		gap: var(--space-5);
		box-shadow: var(--shadow-lg);
		position: relative;
		text-align: left;
	}
	.modal-header {
		display: flex;
		align-items: center;
		gap: var(--space-4);

		:global(.modal-icon.error) {
			color: var(--danger);
		}
		h2 {
			margin: 0;
			font-size: var(--fs-h3);
			color: var(--text-primary);
		}
	}
	.modal-body {
		display: flex;
		flex-direction: column;
		gap: var(--space-4);

		p {
			margin: 0;
			color: var(--text-secondary);
			line-height: var(--lh-snug);
			font-size: var(--fs-sm);
		}

		.input-group {
			display: flex;
			flex-direction: column;
			gap: var(--space-2);
			margin-top: var(--space-2);

			label {
				font-size: var(--fs-sm);
				color: var(--text-secondary);
				font-weight: var(--fw-medium);
			}

			.input-wrapper {
				position: relative;

				input {
					width: 100%;
					background: var(--bg-input);
					border: 1px solid var(--border-default);
					padding: 0.75rem 0.95rem;
					padding-right: 6rem;
					border-radius: var(--radius-sm);
					color: var(--text-primary);
					font-family: var(--font-mono);
					font-size: var(--fs-body);
					box-sizing: border-box;

					&:focus {
						outline: none;
						border-color: var(--primary);
						box-shadow: 0 0 0 3px var(--primary-glow);
					}
				}

				.badge {
					position: absolute;
					right: var(--space-2);
					top: 50%;
					transform: translateY(-50%);
					background: rgba(61, 220, 151, 0.12);
					color: var(--success);
					font-size: var(--fs-xs);
					padding: 0.2rem 0.5rem;
					border-radius: var(--radius-sm);
					border: 1px solid rgba(61, 220, 151, 0.25);
					pointer-events: none;
				}
			}

			.hint {
				font-size: var(--fs-xs);
				color: var(--text-muted);
				margin-top: var(--space-1);
				font-style: italic;
			}
		}
	}
	.modal-actions {
		display: grid;
		grid-template-columns: 1fr 1.5fr;
		gap: var(--space-4);
		margin-top: var(--space-2);

		button {
			padding: 0.85rem;
			border-radius: var(--radius-pill);
			font-weight: var(--fw-semibold);
			font-family: inherit;
			cursor: pointer;
			display: flex;
			align-items: center;
			justify-content: center;
			gap: var(--space-2);
			transition: background var(--dur) var(--ease), filter var(--dur) var(--ease);
			border: 1px solid transparent;

			&.cancel-btn {
				background: var(--tint-soft);
				border-color: var(--border-default);
				color: var(--text-primary);
				&:hover {
					background: var(--tint-softer);
					border-color: var(--border-strong);
				}
			}

			&.confirm-delete-btn {
				background: var(--accent-gradient);
				color: #fff;
				box-shadow: 0 6px 20px -6px var(--primary-glow);
				&:hover {
					filter: brightness(1.06);
				}
				&:disabled {
					opacity: 0.5;
					cursor: not-allowed;
				}
			}
		}
	}
</style>
