<script>
	import { page } from '$app/stores';
	import { onMount } from 'svelte';
	import axios from 'axios';
	import sodium from 'libsodium-wrappers-sumo';
	import { decryptChunk, deriveKeyFromPassword } from '$lib/chacha.js';
	import { toast } from 'svelte-sonner';
	import Icon from '$lib/ui/Icon.svelte';
	import Navbar from '$lib/components/Navbar.svelte';
	import Footer from '$lib/components/Footer.svelte';
	import { shadowKey } from '$lib/stores/shadow.js';
	import { Button, Progress, PasswordInput, Input, Modal, Badge, Spinner } from '$lib/ui';

	function fmtSize(bytes) {
		if (!bytes) return '0 B';
		const k = 1024;
		const s = ['B', 'KB', 'MB', 'GB', 'TB'];
		const i = Math.min(Math.floor(Math.log(bytes) / Math.log(k)), s.length - 1);
		return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + s[i];
	}
	function fmtDate(d) {
		try {
			return new Date(d).toLocaleDateString(undefined, { month: 'short', day: 'numeric', year: 'numeric' });
		} catch {
			return '';
		}
	}
	function typeIcon(mime, name) {
		const m = mime || '';
		if (m.includes('image')) return 'ri:image-line';
		if (m.includes('video')) return 'ri:film-line';
		if (m.includes('audio')) return 'ri:music-2-line';
		if (m.includes('pdf') || m.includes('document')) return 'ri:file-text-line';
		if (/\.(zip|rar|7z|tar|gz)$/i.test(name || '')) return 'ri:file-zip-line';
		return 'ri:file-3-line';
	}

	const CHUNK_SIZE = 100 * 1024 * 1024; // 100MB

	// Server-rendered Open Graph data (from +page.server.js) for link previews.
	let { data } = $props();
	const og = $derived(
		data?.og || {
			title: 'Secure download on Silocat',
			description: 'A file shared securely and stored on silo.cat.'
		}
	);
	const shareUrl = $derived(`https://silo.cat/${$page.params.slug}`);

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
	<title>{og.title}</title>
	<meta name="description" content={og.description} />
	<!-- Private share link: never index or follow (but link previews still render). -->
	<meta name="robots" content="noindex, nofollow" />

	<!-- Open Graph -->
	<meta property="og:type" content="website" />
	<meta property="og:site_name" content="Silocat" />
	<meta property="og:title" content={og.title} />
	<meta property="og:description" content={og.description} />
	<meta property="og:url" content={shareUrl} />
	<meta property="og:image" content="https://silo.cat/og-image.png" />
	<meta property="og:image:width" content="1200" />
	<meta property="og:image:height" content="630" />

	<!-- Twitter -->
	<meta name="twitter:card" content="summary_large_image" />
	<meta name="twitter:title" content={og.title} />
	<meta name="twitter:description" content={og.description} />
	<meta name="twitter:image" content="https://silo.cat/og-image.png" />
</svelte:head>

<div class="dl-page">
	<Navbar />

	<main class="dl-main">
		<div class="dl-card">
			{#if error}
				<div class="state">
					<div class="state-glyph danger"><Icon icon="ri:error-warning-line" width="24" /></div>
					<h1>Link unavailable</h1>
					<p class="state-msg">{error}</p>
					<Button variant="ghost" href="/">Go to Silocat</Button>
				</div>
			{:else if isFolder}
				<div class="head">
					<div class="glyph"><Icon icon="ri:folder-3-line" width="22" /></div>
					<div class="head-text">
						<h1 title={folderMeta.name}>{folderMeta.uploaded_as_files ? 'Shared files' : folderMeta.name}</h1>
						<p class="sub">{folderFiles.length} item{folderFiles.length !== 1 ? 's' : ''} · {fmtSize(folderFiles.reduce((a, f) => a + f.size, 0))}</p>
					</div>
				</div>

				{#if folderFiles.some((f) => f.encrypted)}
					<PasswordInput bind:value={password} label="Password" placeholder="Unlock password" />
				{/if}

				<div class="actions">
					<Button
						block
						loading={isDownloading}
						disabled={folderFiles.some((f) => f.encrypted) && !password}
						onclick={() => startDownloadZip()}
					>
						{#if isDownloading}Preparing zip… {Math.round(progress)}%{:else}<Icon icon="ri:file-zip-line" width="16" /> Download all as zip{/if}
					</Button>
					<button class="del" onclick={handleDeleteClick} aria-label="Delete folder" title="Delete folder">
						<Icon icon="ri:delete-bin-line" width="17" />
					</button>
				</div>
				{#if isDownloading}<Progress value={progress} size="sm" />{/if}

				<div class="file-list">
					{#each folderFiles as file (file.id)}
						<div class="frow">
							<span class="fic"><Icon icon={typeIcon(file.mime, file.name)} width="16" /></span>
							<div class="fmeta">
								<span class="fname" title={file.name}>{file.name}</span>
								<span class="fsize">{fmtSize(file.size)}</span>
							</div>
							{#if file.total_chunks && file.uploaded_chunks < file.total_chunks}
								<span class="uploading"><Icon icon="svg-spinners:12-dots-scale-rotate" width="13" /> {Math.round((file.uploaded_chunks / file.total_chunks) * 100)}%</span>
							{:else if isDownloading && currentDownloadId === file.id}
								<span class="dlpct">{Math.round(currentDownloadProgress)}%</span>
							{/if}
							<button
								class="frow-dl"
								onclick={() => startDownloadSingle(file)}
								disabled={(file.encrypted && !password) || (isDownloading && currentDownloadId === file.id)}
								aria-label="Download file"
							>
								{#if isDownloading && currentDownloadId === file.id}<Spinner size={15} />{:else}<Icon icon="ri:download-line" width="16" />{/if}
							</button>
						</div>
					{/each}
				</div>

				<dl class="meta">
					<div><dt>Type</dt><dd>{folderMeta.uploaded_as_files ? 'File collection' : 'Folder'}</dd></div>
					<div><dt>Items</dt><dd>{folderFiles.length}</dd></div>
					<div><dt>Created</dt><dd>{fmtDate(folderMeta.created_on)}</dd></div>
					<div><dt>Total size</dt><dd>{fmtSize(folderFiles.reduce((a, f) => a + f.size, 0))}</dd></div>
				</dl>
			{:else if fileMeta}
				<div class="head">
					<div class="glyph"><Icon icon={typeIcon(fileMeta.mime, fileMeta.name)} width="22" /></div>
					<div class="head-text">
						<h1 title={fileMeta.name}>{fileMeta.name}</h1>
						<p class="sub">
							{fmtSize(fileMeta.size)}
							{#if fileMeta.encrypted}<span class="dot">·</span><Badge tone="neutral" icon="ri:lock-2-line">Encrypted</Badge>{/if}
						</p>
					</div>
				</div>

				{#if fileMeta.encrypted}
					<PasswordInput bind:value={password} label="Password" placeholder="Unlock password" />
				{/if}

				{#if isDownloading}
					<div class="dl-progress">
						<Progress value={progress} size="md" />
						<div class="dl-progress-row">
							<span>{fileMeta.encrypted ? 'Decrypting' : 'Downloading'} · {Math.round(progress)}%</span>
							{#if isWaiting}
								<span class="waiting"><Icon icon="svg-spinners:12-dots-scale-rotate" width="12" /> waiting for upload {Math.round(uploadProgress)}%</span>
							{:else if uploadProgress < 100}
								<span class="muted">uploaded {Math.round(uploadProgress)}%</span>
							{/if}
						</div>
					</div>
				{:else}
					<div class="actions">
						<Button block disabled={fileMeta.encrypted && !password} onclick={() => startDownload()}>
							{#if fileMeta.encrypted}<Icon icon="ri:lock-unlock-line" width="16" /> Decrypt & download{:else}<Icon icon="ri:download-line" width="16" /> Download{/if}
						</Button>
						<button class="del" onclick={handleDeleteClick} aria-label="Delete file" title="Delete file">
							<Icon icon="ri:delete-bin-line" width="17" />
						</button>
					</div>
				{/if}

				<dl class="meta">
					<div><dt>Size</dt><dd>{fmtSize(fileMeta.size)}</dd></div>
					<div><dt>Uploaded</dt><dd>{fmtDate(fileMeta.created_on)}</dd></div>
					<div><dt>Type</dt><dd class="ellipsis">{fileMeta.mime || 'file'}</dd></div>
					{#if fileMeta.sha256_checksum}
						<div class="full"><dt>SHA-256</dt><dd class="mono ellipsis">{fileMeta.sha256_checksum}</dd></div>
					{/if}
				</dl>
			{:else}
				<div class="state">
					<Spinner size={24} />
					<p class="state-msg">Loading…</p>
				</div>
			{/if}

			{#if !error}
				<p class="secure-note">
					<Icon icon="ri:shield-check-line" width="13" />
					{#if isFolder ? folderFiles.some((f) => f.encrypted) : fileMeta?.encrypted}Encrypted end to end · shared via Silocat{:else}Shared via Silocat{/if}
				</p>
			{/if}
		</div>
	</main>

	<Footer />
</div>

<Modal open={showDeleteModal} title={`Delete ${isFolder ? 'folder' : 'file'}`} icon="ri:delete-bin-line" onclose={() => (showDeleteModal = false)}>
	<div class="del-body">
		<p class="del-msg">This permanently deletes the {isFolder ? 'folder' : 'file'}. It can't be undone.</p>
		<Input bind:value={deleteKeyInput} label="Owner key" icon="ri:key-2-line" placeholder="API key to verify ownership" mono hint={`Required to prove ownership. Without it the ${isFolder ? 'folder' : 'file'} can't be deleted.`}>
			{#if $page.data.user && deleteKeyInput === $page.data.user.api_key}
				<Badge tone="accent">Account</Badge>
			{:else if $shadowKey && deleteKeyInput === $shadowKey}
				<Badge tone="neutral">Browser</Badge>
			{/if}
		</Input>
	</div>
	{#snippet footer()}
		<Button variant="quiet" onclick={() => (showDeleteModal = false)}>Cancel</Button>
		<Button variant="danger-solid" loading={isDeleting} disabled={!deleteKeyInput} onclick={performDelete}>Delete</Button>
	{/snippet}
</Modal>

<style lang="scss">
	.dl-page {
		min-height: 100vh;
		display: flex;
		flex-direction: column;
	}
	.dl-main {
		flex: 1;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: var(--space-8) var(--gutter);
	}
	.dl-card {
		width: 100%;
		max-width: 460px;
		background: var(--surface);
		border: 1px solid var(--edge);
		border-radius: var(--radius-lg);
		padding: var(--space-6);
		display: flex;
		flex-direction: column;
		gap: var(--space-5);
	}

	/* states */
	.state {
		display: flex;
		flex-direction: column;
		align-items: center;
		text-align: center;
		gap: var(--space-3);
		padding: var(--space-6) 0;
		color: var(--ink-faint);

		h1 {
			font-size: var(--fs-h3);
			color: var(--ink);
		}
		.state-msg {
			font-size: var(--fs-sm);
			color: var(--ink-mute);
			margin: 0;
		}
	}
	.state-glyph {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 44px;
		height: 44px;
		border-radius: var(--radius-full);
		border: 1px solid var(--edge);
		color: var(--ink-faint);
		&.danger {
			color: var(--danger);
			border-color: transparent;
			background: var(--danger-soft);
		}
	}

	/* header */
	.head {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		min-width: 0;
	}
	.glyph {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 48px;
		height: 48px;
		border: 1px solid var(--edge);
		border-radius: var(--radius-md);
		color: var(--ink-mute);
		flex-shrink: 0;
	}
	.head-text {
		min-width: 0;
		h1 {
			font-size: var(--fs-body);
			font-weight: var(--fw-semibold);
			overflow: hidden;
			text-overflow: ellipsis;
			white-space: nowrap;
		}
		.sub {
			display: flex;
			align-items: center;
			gap: var(--space-2);
			margin: var(--space-1) 0 0;
			font-family: var(--font-mono);
			font-size: var(--fs-xs);
			color: var(--ink-faint);
		}
		.dot {
			opacity: 0.5;
		}
	}

	/* actions */
	.actions {
		display: flex;
		align-items: center;
		gap: var(--space-2);
	}
	.del {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 40px;
		height: 40px;
		flex-shrink: 0;
		background: transparent;
		border: 1px solid var(--edge);
		border-radius: var(--radius-md);
		color: var(--ink-mute);
		cursor: pointer;
		transition:
			border-color var(--dur) var(--ease),
			color var(--dur) var(--ease),
			background var(--dur) var(--ease);
		&:hover {
			border-color: var(--danger);
			color: var(--danger);
			background: var(--danger-soft);
		}
	}

	.dl-progress {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}
	.dl-progress-row {
		display: flex;
		justify-content: space-between;
		font-family: var(--font-mono);
		font-size: var(--fs-xs);
		color: var(--ink-mute);
		.muted {
			color: var(--ink-faint);
		}
		.waiting {
			display: inline-flex;
			align-items: center;
			gap: 4px;
			color: var(--warn);
		}
	}

	/* folder file list */
	.file-list {
		display: flex;
		flex-direction: column;
		border: 1px solid var(--edge);
		border-radius: var(--radius-md);
		overflow: hidden;
		max-height: 280px;
		overflow-y: auto;
	}
	.frow {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		padding: var(--space-2) var(--space-3);
		& + .frow {
			border-top: 1px solid var(--edge);
		}
	}
	.fic {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 30px;
		height: 30px;
		border: 1px solid var(--edge);
		border-radius: var(--radius-sm);
		color: var(--ink-faint);
		flex-shrink: 0;
	}
	.fmeta {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		.fname {
			font-size: var(--fs-sm);
			color: var(--ink);
			overflow: hidden;
			text-overflow: ellipsis;
			white-space: nowrap;
		}
		.fsize {
			font-family: var(--font-mono);
			font-size: var(--fs-xs);
			color: var(--ink-faint);
		}
	}
	.uploading {
		display: inline-flex;
		align-items: center;
		gap: 3px;
		font-family: var(--font-mono);
		font-size: var(--fs-xs);
		color: var(--warn);
	}
	.dlpct {
		font-family: var(--font-mono);
		font-size: var(--fs-xs);
		color: var(--ink-mute);
	}
	.frow-dl {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 30px;
		height: 30px;
		flex-shrink: 0;
		background: none;
		border: none;
		border-radius: var(--radius-sm);
		color: var(--ink-mute);
		cursor: pointer;
		&:hover:not(:disabled) {
			background: var(--tint-soft);
			color: var(--ink);
		}
		&:disabled {
			opacity: 0.4;
			cursor: not-allowed;
		}
	}

	/* metadata */
	.meta {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: var(--space-3) var(--space-4);
		margin: 0;
		padding-top: var(--space-4);
		border-top: 1px solid var(--edge);

		div {
			min-width: 0;
		}
		.full {
			grid-column: 1 / -1;
		}
		dt {
			font-size: var(--fs-xs);
			color: var(--ink-faint);
			margin-bottom: 2px;
		}
		dd {
			margin: 0;
			font-size: var(--fs-sm);
			color: var(--ink);
			&.mono {
				font-family: var(--font-mono);
				font-size: var(--fs-xs);
			}
			&.ellipsis {
				overflow: hidden;
				text-overflow: ellipsis;
				white-space: nowrap;
			}
		}
	}

	.secure-note {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: var(--space-1);
		font-size: var(--fs-xs);
		color: var(--ink-faint);
		margin: 0;
	}

	/* delete modal body */
	.del-body {
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
	}
	.del-msg {
		font-size: var(--fs-sm);
		color: var(--ink-mute);
		margin: 0;
	}
</style>
