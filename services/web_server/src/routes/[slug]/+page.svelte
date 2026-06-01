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
	:global(body) {
		margin: 0;
		background-color: #0b0b0d;
		color: white;
		font-family: 'Outfit', sans-serif;
	}

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
		padding: 2rem;
		.hero-content {
			text-align: center;
			max-width: 800px;
			width: 100%;
			display: flex;
			flex-direction: column;
			align-items: center;
			gap: 1.5rem;
			h1 {
				font-size: 4rem;
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
		}
	}

	.download-card {
		background: rgba(20, 20, 22, 0.6);
		backdrop-filter: blur(20px);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: 24px;
		padding: 3rem;
		width: 100%;
		max-width: 500px;
		box-shadow: 0 20px 40px rgba(0, 0, 0, 0.3);
		min-height: 300px;
		display: flex;
		align-items: center;
		justify-content: center;

		.state-container {
			display: flex;
			flex-direction: column;
			align-items: center;
			gap: 1rem;
			.state-icon {
				font-size: 3rem;
				color: var(--primary, #ff4655);
			}
			h3 {
				font-size: 1.5rem;
				font-weight: 600;
				margin: 0;
			}
			p {
				color: #a1a1aa;
			}
			&.loading .state-icon {
				color: #a1a1aa;
			}
		}

		.file-details {
			width: 100%;
			display: flex;
			flex-direction: column;
			align-items: center;
			gap: 0.5rem;
			.icon-circle {
				width: 80px;
				height: 80px;
				background: rgba(255, 255, 255, 0.05);
				border-radius: 50%;
				display: flex;
				align-items: center;
				justify-content: center;
				color: var(--primary, #ff4655);
				margin-bottom: 1rem;
			}
			.filename {
				font-size: 1.5rem;
				font-weight: 600;
				margin: 0;
				word-break: break-all;
			}
			.filesize {
				color: #71717a;
				font-size: 1rem;
				margin-bottom: 2rem;
			}

			.action-header {
				display: flex;
				justify-content: space-between;
				align-items: center;
				width: 100%;
				margin-bottom: 1.5rem;

				.filesize {
					color: #71717a;
					font-size: 1rem;
					margin: 0;
				}

				.zip-btn {
					background: rgba(255, 255, 255, 0.1);
					border: 1px solid rgba(255, 255, 255, 0.2);
					color: white;
					padding: 0.5rem 1rem;
					border-radius: 8px;
					font-size: 0.9rem;
					font-weight: 500;
					cursor: pointer;
					display: flex;
					align-items: center;
					gap: 0.5rem;
					transition: all 0.2s;

					&:hover:not(:disabled) {
						background: var(--primary, #ff4655);
						border-color: var(--primary, #ff4655);
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
				gap: 0.75rem;
				margin-bottom: 2rem;
				max-height: 400px;
				overflow-y: auto;
				padding-right: 0.5rem; /* For scrollbar space */

				/* Custom scrollbar */
				&::-webkit-scrollbar {
					width: 6px;
				}
				&::-webkit-scrollbar-track {
					background: rgba(255, 255, 255, 0.02);
				}
				&::-webkit-scrollbar-thumb {
					background: rgba(255, 255, 255, 0.1);
					border-radius: 3px;
				}

				.file-item {
					display: flex;
					justify-content: space-between;
					align-items: center;
					padding: 1rem;
					background: rgba(255, 255, 255, 0.03);
					border: 1px solid rgba(255, 255, 255, 0.05);
					border-radius: 12px;
					transition: all 0.2s;

					&:hover {
						background: rgba(255, 255, 255, 0.07);
						border-color: rgba(255, 255, 255, 0.1);
					}

					.file-info {
						display: flex;
						align-items: center;
						gap: 1rem;
						overflow: hidden;

						.file-icon {
							font-size: 1.5rem;
							color: #a1a1aa;
							flex-shrink: 0;
						}

						.file-text {
							display: flex;
							flex-direction: column;
							overflow: hidden;

							.name {
								font-weight: 500;
								white-space: nowrap;
								overflow: hidden;
								text-overflow: ellipsis;
								max-width: 200px;
							}

							.size {
								font-size: 0.8rem;
								color: #71717a;
							}
						}
					}

					.mini-download-btn {
						background: rgba(255, 255, 255, 0.1);
						border: none;
						color: white;
						width: 36px;
						height: 36px;
						border-radius: 8px;
						display: flex;
						align-items: center;
						justify-content: center;
						cursor: pointer;
						transition: all 0.2s;

						&:hover:not(:disabled) {
							background: var(--primary, #ff4655);
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
				background: rgba(255, 255, 255, 0.05);
				padding: 1.5rem;
				border-radius: 12px;
				display: flex;
				flex-direction: column;
				gap: 0.75rem;
				margin-bottom: 1.5rem;
				border: 1px solid rgba(255, 255, 255, 0.1);

				.input-label {
					display: flex;
					align-items: center;
					gap: 0.5rem;
					color: #a1a1aa;
					font-size: 0.9rem;
					font-weight: 500;
				}
				.password-input {
					width: 100%;
					background: rgba(0, 0, 0, 0.3);
					border: 1px solid rgba(255, 255, 255, 0.1);
					color: white;
					padding: 0.8rem;
					border-radius: 8px;
					font-size: 1rem;
					outline: none;
					transition: border-color 0.2s;
					box-sizing: border-box;
					&:focus {
						border-color: var(--primary, #ff4655);
					}
				}
			}

			.info-badge {
				background: rgba(34, 197, 94, 0.1);
				color: #22c55e;
				padding: 0.6rem 1rem;
				border-radius: 20px;
				font-size: 0.9rem;
				display: flex;
				align-items: center;
				gap: 0.5rem;
				margin-bottom: 2rem;
			}

			.action-area {
				width: 100%;
				.download-btn {
					width: 100%;
					background: var(--primary, #ff4655);
					color: white;
					border: none;
					padding: 1rem;
					border-radius: 12px;
					font-weight: 600;
					font-size: 1.1rem;
					cursor: pointer;
					display: flex;
					align-items: center;
					justify-content: center;
					gap: 0.75rem;
					transition: all 0.2s;
					&:hover {
						background: #e03e4b;
						transform: translateY(-2px);
					}
					&:disabled {
						opacity: 0.5;
						cursor: not-allowed;
						transform: none;
						background: #52525b;
					}
				}

				.progress-container {
					width: 100%;
					.progress-bar-bg {
						height: 8px;
						background: rgba(255, 255, 255, 0.1);
						border-radius: 4px;
						overflow: hidden;
						margin-bottom: 0.5rem;
						position: relative;
						.progress-bar-fill {
							height: 100%;
							background: var(--primary, #ff4655);
							transition: width 0.3s ease;
						}
					}
					.progress-text {
						font-size: 0.9rem;
						color: #a1a1aa;
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
		z-index: 1;
		.glow-spot {
			position: absolute;
			width: 600px;
			height: 600px;
			background: var(--primary, #ff4655);
			filter: blur(120px);
			opacity: 0.08;
			border-radius: 50%;
			&.top {
				top: -200px;
				left: 20%;
				transform: translateX(-50%);
			}
			&.bottom {
				bottom: -200px;
				right: 20%;
				transform: translateX(50%);
				width: 800px;
				height: 800px;
				opacity: 0.05;
				background: #4f46e5;
			}
		}
	}
	.metadata-section {
		width: 100%;
		margin-top: 2rem;
		padding-top: 2rem;
		border-top: 1px solid rgba(255, 255, 255, 0.1);

		h3 {
			font-size: 1.1rem;
			font-weight: 600;
			margin-bottom: 1rem;
			color: white;
		}

		.meta-grid {
			display: grid;
			grid-template-columns: repeat(2, 1fr);
			gap: 1rem;

			.meta-item {
				display: flex;
				flex-direction: column;
				gap: 0.25rem;

				&.full-width {
					grid-column: span 2;
				}

				.label {
					font-size: 0.8rem;
					color: #71717a;
					text-transform: uppercase;
					letter-spacing: 0.05em;
				}

				.value {
					font-size: 0.95rem;
					color: #e4e4e7;
					word-break: break-all;

					&.mono {
						font-family: monospace;
						font-size: 0.85rem;
						color: #a1a1aa;
					}
				}
			}
		}
	}
	.action-area {
		display: flex;
		gap: 1rem;
		width: 100%;

		.download-btn {
			flex: 1;
		}

		.delete-btn {
			background: rgba(239, 68, 68, 0.1);
			border: 1px solid rgba(239, 68, 68, 0.2);
			color: #ef4444;
			padding: 0 1.5rem;
			border-radius: 12px;
			cursor: pointer;
			transition: all 0.2s;
			display: flex;
			align-items: center;
			justify-content: center;

			&:hover {
				background: rgba(239, 68, 68, 0.2);
				border-color: rgba(239, 68, 68, 0.5);
				color: #f87171;
			}
		}
	}

	.modal-backdrop {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.8);
		backdrop-filter: blur(8px);
		z-index: 100;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 1rem;
	}
	.modal-content {
		background: #18181b;
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: 24px;
		padding: 2rem;
		width: 100%;
		max-width: 420px;
		display: flex;
		flex-direction: column;
		gap: 1.5rem;
		box-shadow: 0 25px 50px -12px rgba(0, 0, 0, 0.5);
		position: relative;
		text-align: left;
	}
	.modal-header {
		display: flex;
		align-items: center;
		gap: 1rem;

		:global(.modal-icon.error) {
			color: #ef4444;
		}
		h2 {
			margin: 0;
			font-size: 1.5rem;
			color: #f4f4f5;
		}
	}
	.modal-body {
		display: flex;
		flex-direction: column;
		gap: 1rem;

		p {
			margin: 0;
			color: #a1a1aa;
			line-height: 1.5;
			font-size: 0.95rem;
		}

		.input-group {
			display: flex;
			flex-direction: column;
			gap: 0.5rem;
			margin-top: 0.5rem;

			label {
				font-size: 0.85rem;
				color: #d4d4d8;
				font-weight: 500;
			}

			.input-wrapper {
				position: relative;

				input {
					width: 100%;
					background: #09090b;
					border: 1px solid rgba(255, 255, 255, 0.1);
					padding: 0.75rem 1rem;
					padding-right: 6rem;
					border-radius: 12px;
					color: white;
					font-family: monospace;
					font-size: 1rem;
					box-sizing: border-box;

					&:focus {
						outline: none;
						border-color: var(--primary, #ff4655);
						box-shadow: 0 0 0 2px rgba(255, 70, 85, 0.2);
					}
				}

				.badge {
					position: absolute;
					right: 0.5rem;
					top: 50%;
					transform: translateY(-50%);
					background: rgba(34, 197, 94, 0.1);
					color: #4ade80;
					font-size: 0.7rem;
					padding: 0.2rem 0.5rem;
					border-radius: 6px;
					border: 1px solid rgba(34, 197, 94, 0.2);
					pointer-events: none;
				}
			}

			.hint {
				font-size: 0.8rem;
				color: #71717a;
				margin-top: 0.25rem;
				font-style: italic;
			}
		}
	}
	.modal-actions {
		display: grid;
		grid-template-columns: 1fr 1.5fr;
		gap: 1rem;
		margin-top: 0.5rem;

		button {
			padding: 0.875rem;
			border-radius: 12px;
			font-weight: 600;
			cursor: pointer;
			display: flex;
			align-items: center;
			justify-content: center;
			gap: 0.5rem;
			transition: all 0.2s;
			border: none;

			&.cancel-btn {
				background: rgba(255, 255, 255, 0.05);
				color: #d4d4d8;
				&:hover {
					background: rgba(255, 255, 255, 0.1);
					color: white;
				}
			}

			&.confirm-delete-btn {
				background: #ef4444;
				color: white;
				&:hover {
					background: #dc2626;
					box-shadow: 0 4px 12px rgba(239, 68, 68, 0.3);
				}
				&:disabled {
					opacity: 0.5;
					cursor: not-allowed;
				}
			}
		}
	}
</style>
