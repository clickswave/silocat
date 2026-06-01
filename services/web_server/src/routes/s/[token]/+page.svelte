<script>
	import { page } from '$app/stores';
	import Icon from '@iconify/svelte';
	import axios from 'axios';
	import { onMount } from 'svelte';
	import { fade, scale } from 'svelte/transition';
	import { toast } from 'svelte-sonner';
	import { deriveKeyFromPassword, decryptChunk } from '$lib/chacha.js';
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
				// Check encryption
				if (file.type === 'file' && file.encrypted) {
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
			// Authorize download and get chunks
			const res = await axios.post('/api/v1/public/share/authorize', { token });
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
			if (e.message.includes('Wrong password') || e.message.includes('Decryption failed')) {
				toast.error('Incorrect password.');
			} else if (e.response && e.response.status === 410) {
				error = "This 'Once' link has expired.";
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
				file_id: f.id
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
					file_id: f.id
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
</script>

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
												<Icon icon="ri:lock-fill" width="12" style="color: #fbbf24;" />
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
					<span>Securely shared via SiloCat</span>
				</div>
			</div>
		{/if}
	</div>
</div>

<style lang="scss">
	.page-container {
		height: 100vh;
		width: 100vw;
		display: flex;
		align-items: center;
		justify-content: center;
		background: radial-gradient(circle at center, #1f2937, #0f1216);
		color: #fff;
		font-family: 'Inter', sans-serif;
	}

	.center-card {
		background: rgba(255, 255, 255, 0.03);
		backdrop-filter: blur(10px);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: 24px;
		padding: 40px;
		width: 90%;
		max-width: 500px;
		text-align: center;
		box-shadow: 0 20px 60px rgba(0, 0, 0, 0.5);
		display: flex;
		flex-direction: column;
		max-height: 90vh;
	}

	.scroll-area {
		overflow-y: auto;
		flex: 1;
		margin-top: 20px;
		margin-bottom: 20px;
		text-align: left;
		background: rgba(0, 0, 0, 0.2);
		border-radius: 12px;
		padding: 10px;
	}

	.file-item {
		display: flex;
		align-items: center;
		padding: 8px;
		border-bottom: 1px solid rgba(255, 255, 255, 0.05);
		gap: 10px;

		&:last-child {
			border-bottom: none;
		}
	}

	.file-icon {
		color: #aaa;
	}

	.file-info {
		flex: 1;
		min-width: 0;
	}

	.file-name {
		font-size: 14px;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		color: #eee;
	}

	.file-meta {
		font-size: 12px;
		color: #666;
		display: flex;
		align-items: center;
		gap: 6px;
	}

	.icon-circle {
		width: 64px;
		height: 64px;
		border-radius: 50%;
		display: flex;
		align-items: center;
		justify-content: center;
		margin: 0 auto 20px;
		flex-shrink: 0;

		&.primary {
			background: rgba(255, 70, 85, 0.1);
			color: #ff4655;
		}
		&.error {
			background: rgba(239, 68, 68, 0.1);
			color: #ef4444;
		}
	}

	h1,
	h2 {
		font-weight: 600;
		margin: 0 0 8px;
		color: #eee;
	}

	h1 {
		font-size: 20px;
		word-break: break-word;
	}
	h2 {
		font-size: 24px;
	}

	p {
		color: #888;
		font-size: 14px;
		margin: 0;

		&.meta {
			margin-bottom: 8px;
			font-family: 'JetBrains Mono', monospace;
			background: rgba(255, 255, 255, 0.05);
			display: inline-block;
			padding: 4px 8px;
			border-radius: 4px;
		}
	}

	.encrypted-badge {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 6px;
		color: #fbbf24;
		font-size: 12px;
		margin-top: 4px;
	}

	.actions {
		margin-top: 10px;
		min-height: 50px;
		flex-shrink: 0;
	}

	.password-group {
		display: flex;
		gap: 8px;
		margin-bottom: 10px;

		input {
			flex: 1;
			background: rgba(0, 0, 0, 0.3);
			border: 1px solid rgba(255, 255, 255, 0.1);
			border-radius: 12px;
			padding: 10px 14px;
			color: #ccc;
			font-size: 14px;
			min-width: 0;

			&:focus {
				outline: none;
				border-color: var(--primary, #ff4655);
			}
		}
	}

	.file-action-btn {
		background: rgba(255, 255, 255, 0.1);
		border: none;
		color: #eee;
		width: 32px;
		height: 32px;
		display: flex;
		align-items: center;
		justify-content: center;
		border-radius: 8px;
		cursor: pointer;
		transition: all 0.2s;

		&:hover {
			background: rgba(255, 255, 255, 0.2);
			color: #fff;
		}
	}

	.download-btn {
		background: #ff4655;
		color: white;
		border: none;
		width: 100%;
		padding: 12px;
		border-radius: 12px;
		font-size: 16px;
		font-weight: 600;
		cursor: pointer;
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 8px;
		transition: all 0.2s;

		&:hover {
			background: #e11d48;
			transform: translateY(-2px);
			box-shadow: 0 4px 12px rgba(255, 70, 85, 0.4);
		}

		&:disabled {
			opacity: 0.7;
			cursor: not-allowed;
			transform: none;
		}

		&.small {
			width: auto;
			padding: 10px 16px;
		}
	}

	.spinner {
		animation: spin 1s linear infinite;
	}

	.secure-note {
		margin-top: 24px;
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 6px;
		font-size: 12px;
		color: #555;
		flex-shrink: 0;
	}

	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
	}
</style>
