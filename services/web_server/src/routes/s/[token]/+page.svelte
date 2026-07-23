<script>
	import { page } from '$app/stores';
	import Icon from '@iconify/svelte';
	import axios from 'axios';
	import { onMount } from 'svelte';
	import { fade, scale } from 'svelte/transition';
	import { toast } from 'svelte-sonner';
	import { deriveKeyFromPassword, decryptChunk } from '$lib/chacha.js';
	import Prompt from '$lib/ui/Prompt.svelte';
	import sodium from 'libsodium-wrappers-sumo';
	import JSZip from 'jszip';

	let token = $page.params.token;
	let loading = true;
	let error = null;
	let file = null; // { id, name, size, type, data: { encrypted: bool } }
	let downloading = false;
	let downloadProgress = 0;

	// Encryption state
	let needsPassword = false;
	let password = '';
	let showPasswordInput = false;

	// Fetch info on mount
	onMount(async () => {
		try {
			await sodium.ready;
			const res = await axios.get(`/api/v1/public/share/info/${token}`);
			if (res.data.success) {
				file = res.data.success.data;
				// Needs a password if it is client-side encrypted OR the owner set a
				// link password gate (server-enforced on authorize).
				if (file.password_required) {
					needsPassword = true;
				} else if (file.type === 'file' && file.encrypted) {
					needsPassword = true;
				} else if (file.type === 'folder' && file.files && file.files.some((f) => f.encrypted)) {
					needsPassword = true;
				}
			}
		} catch (e) {
			console.error(e);
			if (e.response && e.response.status === 410) {
				error = "This 'Once' link has expired.";
			} else {
				error = 'Invalid or expired link.';
			}
		} finally {
			loading = false;
		}
	});

	async function handleDownload() {
		if (!file) return;

		if (needsPassword && !password) {
			showPasswordInput = true;
			// focus input?
			return;
		}

		downloading = true;
		downloadProgress = 0;

		try {
			// Authorize download and get chunks (password gate enforced server-side).
			const res = await axios.post('/api/v1/public/share/authorize', { token, password });
			if (res.data.success) {
				const data = res.data.success.data;

				if (data.type === 'folder') {
					// Update files from authorized response just in case, though authorize mostly just authorizes.
					// Actually, our authorize response now returns 'files' too.
					await handleFolderDownload(data.files || file.files, file.name);
					return;
				}

				const chunks = data.chunks;
				if (!chunks || chunks.length === 0) {
					toast.error('File appears empty or corrupted.');
					downloading = false;
					return;
				}

				await handleFileDownload(file, chunks, password);
			}
		} catch (e) {
			console.error(e);
			if (e.response && e.response.status === 401) {
				// Server-side link password gate rejected the password.
				showPasswordInput = true;
				password = '';
				toast.error('Incorrect link password.');
			} else if (e.message.includes('Wrong password') || e.message.includes('Decryption failed')) {
				toast.error('Incorrect password.');
			} else if (e.response && e.response.status === 410) {
				error = e.response.data?.message || 'This link has expired.';
			} else {
				toast.error('Unified download failed.');
			}
		} finally {
			downloading = false;
		}
	}

	async function downloadSingleFile(f) {
		if (f.encrypted && !password) {
			toast.error('Please enter the password first');
			showPasswordInput = true;
			return;
		}

		const toastId = toast.loading(`Preparing ${f.name}...`);
		try {
			const chunksRes = await axios.post('/api/v1/public/share/fetch-chunks', {
				token: token,
				file_id: f.id,
				password
			});

			if (chunksRes.data && chunksRes.data.success) {
				const chunks = chunksRes.data.success.data.chunks;
				if (!chunks || chunks.length === 0) {
					toast.error('File empty');
					return;
				}

				await handleFileDownload(f, chunks, password);
				toast.dismiss(toastId);
			} else {
				throw new Error('Failed to fetch info');
			}
		} catch (e) {
			console.error(e);
			toast.error('Download failed');
			toast.dismiss(toastId);
		}
	}

	async function handleFileDownload(fileMeta, chunks, password) {
		// Prepare decryption key if needed
		let fileKey = null;
		if (fileMeta.encrypted) {
			const firstChunk = chunks[0];
			if (!firstChunk.salt) {
				console.warn('Encrypted file missing salt!');
			} else {
				const saltBytes = Uint8Array.from(atob(firstChunk.salt), (c) => c.charCodeAt(0));
				fileKey = await deriveKeyFromPassword(password, saltBytes);
			}
		}

		// Download chunks
		const downloadedChunks = [];
		let completedBytes = 0;

		for (let i = 0; i < chunks.length; i++) {
			const chunk = chunks[i];
			const chunkDataRes = await axios.get(chunk.presigned_url, {
				responseType: 'arraybuffer'
			});

			let dataBytes = new Uint8Array(chunkDataRes.data);

			if (fileMeta.encrypted && fileKey) {
				if (!chunk.nonce) throw new Error(`Chunk ${i} missing nonce`);
				const nonceBytes = Uint8Array.from(atob(chunk.nonce), (c) => c.charCodeAt(0));
				try {
					dataBytes = await decryptChunk(dataBytes, fileKey, nonceBytes);
				} catch (decryptErr) {
					throw new Error('Decryption failed. Wrong password?');
				}
			}

			downloadedChunks.push(dataBytes);
			completedBytes += chunk.size;
			downloadProgress = Math.floor((completedBytes / fileMeta.size) * 100);
		}

		// Assemble
		const blob = new Blob(downloadedChunks, { type: fileMeta.mime });
		const url = URL.createObjectURL(blob);
		const a = document.createElement('a');
		a.href = url;
		a.download = fileMeta.name;
		document.body.appendChild(a);
		a.click();
		document.body.removeChild(a);
		URL.revokeObjectURL(url);

		toast.success('Download complete');
	}

	async function handleFolderDownload(files, folderName) {
		const zip = new JSZip();
		let processedCount = 0;
		const totalFiles = files.length;

		if (totalFiles === 0) {
			toast.error('Folder is empty');
			return;
		}

		// Pre-derive keys for efficiency if possible?
		// But salt is per-file. So we derive per file.

		for (let i = 0; i < files.length; i++) {
			const f = files[i];
			try {
				// Update progress based on file count for simplicity in this view
				// Ideally we track bytes, but files vary in size.
				downloadProgress = Math.floor((i / totalFiles) * 100);

				// Fetch chunks for this file
				const chunksRes = await axios.post('/api/v1/public/share/fetch-chunks', {
					token: token,
					file_id: f.id,
					password
				});

				if (chunksRes.data && chunksRes.data.success) {
					const chunks = chunksRes.data.success.data.chunks;
					if (!chunks || chunks.length === 0) continue;

					// Decrypt setup
					let fileKey = null;
					if (f.encrypted) {
						const firstChunk = chunks[0];
						if (!firstChunk.salt) throw new Error(`File ${f.name} missing salt.`);
						const saltBytes = Uint8Array.from(atob(firstChunk.salt), (c) => c.charCodeAt(0));
						fileKey = await deriveKeyFromPassword(password, saltBytes);
					}

					// Download and decrypt chunks
					const downloadedChunks = [];
					for (const chunk of chunks) {
						const chunkData = await axios.get(chunk.presigned_url, { responseType: 'arraybuffer' });
						let dataBytes = new Uint8Array(chunkData.data);

						if (f.encrypted) {
							if (!chunk.nonce) throw new Error(`Chunk missing nonce for ${f.name}`);
							const nonceBytes = Uint8Array.from(atob(chunk.nonce), (c) => c.charCodeAt(0));
							dataBytes = await decryptChunk(dataBytes, fileKey, nonceBytes);
						}
						downloadedChunks.push(dataBytes);
					}

					const fileBlob = new Blob(downloadedChunks);
					zip.file(f.name, fileBlob);
					processedCount++;
				}
			} catch (e) {
				console.error(`Failed to process file ${f.name}`, e);
				// Continue with other files?
			}
		}

		if (processedCount === 0) {
			throw new Error('No files could be processed successfully');
		}

		downloadProgress = 99;
		const zipContent = await zip.generateAsync({ type: 'blob' });
		const zipName = `${folderName}.zip`;
		const url = URL.createObjectURL(zipContent);

		const a = document.createElement('a');
		a.href = url;
		a.download = zipName;
		document.body.appendChild(a);
		a.click();
		document.body.removeChild(a);
		URL.revokeObjectURL(url);

		downloadProgress = 100;
		toast.success('Folder download complete');
	}

	function formatSize(bytes) {
		if (!bytes) return '0 B';
		const units = ['B', 'KB', 'MB', 'GB', 'TB'];
		let i = 0;
		while (bytes >= 1024 && i < units.length - 1) {
			bytes /= 1024;
			i++;
		}
		return `${bytes.toFixed(1)} ${units[i]}`;
	}

	let reporting = false;
	let showReportModal = false;
	async function submitReport(reason) {
		if (!reason || !reason.trim()) return;
		showReportModal = false;
		reporting = true;
		try {
			await axios.post('/api/v1/public/report', { share_token: token, reason: reason.trim() });
			toast.success('Report submitted. Thank you, our team will review it.');
		} catch {
			toast.error('Could not submit your report. Please try again later.');
		} finally {
			reporting = false;
		}
	}
</script>

<svelte:head>
	<title>Secure download - Silocat</title>
	<!-- Private share link: never index or follow. -->
	<meta name="robots" content="noindex, nofollow" />
</svelte:head>

<div class="page-container">
	<div class="center-card">
		{#if loading}
			<div class="loading">
				<Icon icon="ri:loader-4-line" class="spinner" width="48" />
				<p>Verifying Link...</p>
			</div>
		{:else if error}
			<div class="error" in:scale>
				<div class="icon-circle error">
					<Icon icon="ri:error-warning-fill" width="32" />
				</div>
				<h2>Link Unavailable</h2>
				<p>{error}</p>
			</div>
		{:else}
			<div class="file-preview" in:fade>
				<div class="icon-circle primary">
					{#if file.type === 'folder'}
						<Icon icon="ri:folder-5-fill" width="32" />
					{:else}
						<Icon icon="ri:file-text-fill" width="32" />
					{/if}
				</div>
				<h1>{file.name}</h1>
				{#if file.type === 'file'}
					<p class="meta">{formatSize(file.size)}</p>
					{#if file.encrypted}
						<div class="encrypted-badge">
							<Icon icon="ri:lock-fill" width="14" />
							Encrypted
						</div>
					{/if}
				{:else if file.type === 'folder'}
					<p class="meta">{file.files ? file.files.length : 0} items</p>

					{#if file.files && file.files.length > 0}
						<div class="scroll-area">
							{#each file.files as f}
								<div class="file-item">
									<div class="file-icon">
										<Icon
											icon={f.mime && f.mime.includes('image')
												? 'ri:image-fill'
												: 'ri:file-text-fill'}
											width="20"
										/>
									</div>
									<div class="file-info">
										<div class="file-name">{f.name}</div>
										<div class="file-meta">
											{formatSize(f.size)}
											{#if f.encrypted}
												<Icon icon="ri:lock-fill" width="12" style="color: var(--warn);" />
											{/if}
										</div>
									</div>
									<button
										class="file-action-btn"
										on:click={() => downloadSingleFile(f)}
										title="Download File"
									>
										<Icon icon="ri:download-line" width="18" />
									</button>
								</div>
							{/each}
						</div>
					{/if}
				{/if}

				<div class="actions">
					{#if showPasswordInput || needsPassword}
						<div class="password-group" transition:fade>
							<input
								type="password"
								bind:value={password}
								placeholder="Enter decryption password"
								on:keydown={(e) => e.key === 'Enter' && handleDownload()}
							/>
							<!-- Small download button next to password if space permits, or mainly rely on big button below -->
						</div>
					{/if}

					<button
						class="download-btn"
						on:click={handleDownload}
						disabled={downloading || (needsPassword && !password)}
					>
						{#if downloading}
							<Icon icon="ri:loader-4-line" class="spinner" width="20" />
							{downloadProgress}%
						{:else}
							<Icon
								icon={needsPassword && !password && false ? 'ri:lock-fill' : 'ri:download-2-fill'}
								width="20"
							/>
							{file.type === 'folder' ? 'Download All as Zip' : 'Download'}
						{/if}
					</button>
				</div>
				<div class="secure-note">
					<Icon icon="ri:shield-check-line" width="14" />
					<span>Securely shared via Silocat</span>
				</div>
				<button class="report-link" type="button" on:click={() => (showReportModal = true)} disabled={reporting}>
					<Icon icon="ri:flag-line" width="12" />
					{reporting ? 'Reporting…' : 'Report this link'}
				</button>
			</div>
		{/if}
	</div>
</div>

<Prompt
	open={showReportModal}
	title="Report this link"
	message="Briefly, what is the problem? For example copyright infringement, or illegal or harmful content."
	placeholder="Reason"
	submitLabel="Submit report"
	onsubmit={submitReport}
	onclose={() => (showReportModal = false)}
/>

<style lang="scss">
	.page-container {
		min-height: 100vh;
		width: 100%;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: var(--gutter);
		background:
			radial-gradient(circle at 50% 0%, rgba(255, 70, 85, 0.12) 0%, transparent 55%),
			var(--bg-app);
		color: var(--text-primary);
		font-family: var(--font-sans);
	}

	.center-card {
		background: var(--bg-card);
		border: 1px solid var(--border-default);
		border-radius: var(--radius-lg);
		padding: clamp(1.5rem, 5vw, 2.5rem);
		width: 100%;
		max-width: 500px;
		text-align: center;
		box-shadow: var(--shadow-lg);
		display: flex;
		flex-direction: column;
		max-height: 90vh;
	}

	.scroll-area {
		overflow-y: auto;
		flex: 1;
		margin: var(--space-5) 0;
		text-align: left;
		background: var(--tint-soft);
		border: 1px solid var(--hairline);
		border-radius: var(--radius-sm);
		padding: var(--space-2);
	}

	.file-item {
		display: flex;
		align-items: center;
		padding: var(--space-2);
		border-bottom: 1px solid var(--hairline);
		gap: var(--space-3);

		&:last-child {
			border-bottom: none;
		}
	}

	.file-icon {
		color: var(--text-secondary);
	}

	.file-info {
		flex: 1;
		min-width: 0;
	}

	.file-name {
		font-size: var(--fs-sm);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		color: var(--text-primary);
	}

	.file-meta {
		font-size: var(--fs-xs);
		color: var(--text-muted);
		display: flex;
		align-items: center;
		gap: var(--space-2);
	}

	.icon-circle {
		width: 64px;
		height: 64px;
		border-radius: 50%;
		display: flex;
		align-items: center;
		justify-content: center;
		margin: 0 auto var(--space-5);
		flex-shrink: 0;

		&.primary {
			background: rgba(255, 70, 85, 0.1);
			color: var(--primary);
			box-shadow: var(--shadow-glow);
		}
		&.error {
			background: rgba(255, 70, 85, 0.1);
			color: var(--danger);
		}
	}

	h1,
	h2 {
		font-weight: var(--fw-semibold);
		margin: 0 0 var(--space-2);
		color: var(--text-primary);
	}

	h1 {
		font-size: var(--fs-h3);
		word-break: break-word;
	}
	h2 {
		font-size: var(--fs-h2);
	}

	p {
		color: var(--text-secondary);
		font-size: var(--fs-sm);
		margin: 0;

		&.meta {
			margin-bottom: var(--space-2);
			font-family: var(--font-mono);
			background: var(--tint-soft);
			border: 1px solid var(--hairline);
			display: inline-block;
			padding: 4px 8px;
			border-radius: var(--radius-sm);
		}
	}

	.encrypted-badge {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: var(--space-2);
		color: var(--warning);
		font-size: var(--fs-xs);
		margin-top: var(--space-1);
	}

	.actions {
		margin-top: var(--space-3);
		min-height: 50px;
		flex-shrink: 0;
	}

	.password-group {
		display: flex;
		gap: var(--space-2);
		margin-bottom: var(--space-3);

		input {
			flex: 1;
			background: var(--bg-input);
			border: 1px solid var(--border-default);
			border-radius: var(--radius-sm);
			padding: 0.75rem 0.95rem;
			color: var(--text-primary);
			font-size: var(--fs-sm);
			font-family: var(--font-mono);
			min-width: 0;
			outline: none;
			transition: border-color var(--dur) var(--ease), box-shadow var(--dur) var(--ease);

			&::placeholder {
				color: var(--text-muted);
			}

			&:focus {
				border-color: var(--primary);
				box-shadow: 0 0 0 3px var(--primary-glow);
			}
		}
	}

	.file-action-btn {
		background: var(--tint-softer);
		border: 1px solid var(--border-default);
		color: var(--text-primary);
		width: 32px;
		height: 32px;
		display: flex;
		align-items: center;
		justify-content: center;
		border-radius: var(--radius-sm);
		cursor: pointer;
		flex-shrink: 0;
		transition: background var(--dur) var(--ease);

		&:hover {
			background: var(--bg-card-hover);
		}
	}

	.download-btn {
		background: var(--accent-gradient);
		color: #fff;
		border: none;
		width: 100%;
		padding: 0.85rem;
		border-radius: var(--radius-pill);
		font-size: var(--fs-body);
		font-weight: var(--fw-semibold);
		font-family: inherit;
		cursor: pointer;
		display: flex;
		align-items: center;
		justify-content: center;
		gap: var(--space-2);
		box-shadow: 0 6px 20px -6px var(--primary-glow);
		transition: filter var(--dur) var(--ease), transform var(--dur) var(--ease);

		&:hover {
			filter: brightness(1.06);
			transform: translateY(-1px);
		}

		&:disabled {
			opacity: 0.6;
			cursor: not-allowed;
			transform: none;
		}

		&.small {
			width: auto;
			padding: 0.7rem 1rem;
		}
	}

	.spinner {
		animation: spin 1s linear infinite;
	}

	.secure-note {
		margin-top: var(--space-6);
		display: flex;
		align-items: center;
		justify-content: center;
		gap: var(--space-2);
		font-size: var(--fs-xs);
		color: var(--text-muted);
		flex-shrink: 0;
	}

	.report-link {
		margin-top: var(--space-3);
		display: inline-flex;
		align-items: center;
		gap: var(--space-1);
		background: none;
		border: none;
		cursor: pointer;
		font-size: var(--fs-xs);
		color: var(--text-muted);
		opacity: 0.7;
		transition: opacity 0.15s ease;
	}
	.report-link:hover:not(:disabled) {
		opacity: 1;
		color: var(--text-secondary, var(--text-muted));
	}
	.report-link:disabled {
		cursor: default;
		opacity: 0.5;
	}

	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
	}
</style>
