<script>
	import { glyphForMime } from '$lib/ui/icons.js';
	import Footer from '$lib/components/Footer.svelte';
	import Navbar from '$lib/components/Navbar.svelte';
	import { page } from '$app/stores';
	import Icon from '$lib/ui/Icon.svelte';
	import axios from 'axios';
	import { onMount } from 'svelte';
	import { fade, scale } from 'svelte/transition';
	import { toast } from '$lib/toast.js';
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
					// The authorize response carries the folder's files as well.
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
	<title>Secure download · Silocat</title>
	<!-- Private share link: never index or follow. -->
	<meta name="robots" content="noindex, nofollow" />
</svelte:head>

<div class="page">
	<Navbar />

	<main class="main">
		<div class="card">
			{#if loading}
				<div class="state">
					<Icon name="spinner" size={24} />
					<span class="state-line">Verifying link…</span>
				</div>
			{:else if error}
				<div class="state">
					<span class="glyph danger"><Icon name="alert" size={22} /></span>
					<h1>Link unavailable</h1>
					<p class="state-line">{error}</p>
					<a class="ghost-btn" href="/">Go to Silocat</a>
				</div>
			{:else}
				<div class="head">
					<span class="glyph">
						<Icon name={file.type === 'folder' ? 'folder-wide' : glyphForMime(file.mime, file.name)} size={22} />
					</span>
					<div class="head-text">
						<h1 title={file.name}>{file.name}</h1>
						<p class="sub">
							{#if file.type === 'folder'}
								{file.files ? file.files.length : 0} item{(file.files?.length ?? 0) === 1 ? '' : 's'}
							{:else}
								{formatSize(file.size)}
							{/if}
							{#if file.encrypted}
								<span class="dot">·</span>
								<span class="enc"><Icon name="lock" size={12} stroke={1.9} /> Encrypted</span>
							{/if}
						</p>
					</div>
				</div>

				{#if file.type === 'folder' && file.files && file.files.length > 0}
					<div class="file-list">
						{#each file.files as f (f.id ?? f.name)}
							<div class="frow">
								<span class="fglyph"><Icon name={glyphForMime(f.mime, f.name)} size={16} /></span>
								<div class="fmeta">
									<span class="fname" title={f.name}>{f.name}</span>
									<span class="fsize">{formatSize(f.size)}</span>
								</div>
								{#if f.encrypted}
									<span class="flock"><Icon name="lock" size={13} stroke={1.9} /></span>
								{/if}
								<button
									type="button"
									class="fdl"
									aria-label="Download {f.name}"
									title="Download"
									onclick={() => downloadSingleFile(f)}
								>
									<Icon name="download" size={15} />
								</button>
							</div>
						{/each}
					</div>
				{/if}

				{#if showPasswordInput || needsPassword}
					<div class="pw-field">
						<label for="share-pw">Password</label>
						<input
							id="share-pw"
							type="password"
							bind:value={password}
							placeholder="Decryption password"
							onkeydown={(e) => e.key === 'Enter' && handleDownload()}
						/>
						<span class="pw-hint">
							The sender shared this separately. Without it the file cannot be decrypted.
						</span>
					</div>
				{/if}

				{#if downloading}
					<div class="progress">
						<div class="progress-head">
							<span>{file.encrypted ? 'Decrypting' : 'Downloading'}</span>
							<span class="mono">{downloadProgress}%</span>
						</div>
						<div class="track"><div class="fill" style="width:{downloadProgress}%"></div></div>
					</div>
				{:else}
					<button
						type="button"
						class="primary"
						disabled={needsPassword && !password}
						onclick={handleDownload}
					>
						<Icon name={file.encrypted ? 'unlock' : 'download'} size={16} />
						{file.type === 'folder' ? 'Download all as zip' : file.encrypted ? 'Decrypt & download' : 'Download'}
					</button>
				{/if}

				<p class="secure-note">
					<Icon name="shield-check" size={13} />
					{file.encrypted ? 'Encrypted end to end · shared via Silocat' : 'Shared via Silocat'}
				</p>

				<button
					type="button"
					class="report"
					disabled={reporting}
					onclick={() => (showReportModal = true)}
				>
					<Icon name="flag" size={12} />
					{reporting ? 'Reporting…' : 'Report this link'}
				</button>
			{/if}
		</div>
	</main>

	<Footer />
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
	.page {
		min-height: 100vh;
		display: flex;
		flex-direction: column;
		background: var(--bg);
		color: var(--ink);
		font-family: var(--font-sans);
		font-size: var(--fs-body);
		line-height: var(--lh-normal);
	}

	.main {
		flex: 1;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: clamp(2rem, 6vw, 4rem) var(--gutter);
	}

	.card {
		width: 100%;
		max-width: 440px;
		display: flex;
		flex-direction: column;
		gap: 0.875rem;
		padding: 1.25rem;
		border: 1px solid var(--edge);
		border-radius: var(--radius-lg);
		background: var(--surface);
	}

	.head {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		min-width: 0;
	}

	.glyph {
		display: grid;
		place-items: center;
		width: 40px;
		height: 40px;
		flex: 0 0 auto;
		border-radius: var(--radius-md);
		background: var(--tint-soft);
		color: var(--ink-mute);

		&.danger {
			background: var(--danger-soft);
			color: var(--danger);
		}
	}

	.head-text {
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 0.125rem;

		h1 {
			margin: 0;
			font-size: 1.0625rem;
			font-weight: var(--fw-semibold);
			letter-spacing: var(--tracking-tight);
			overflow: hidden;
			text-overflow: ellipsis;
			white-space: nowrap;
		}
	}

	.sub {
		margin: 0;
		display: flex;
		align-items: center;
		gap: 0.375rem;
		font-family: var(--font-mono);
		font-size: var(--fs-xs);
		color: var(--ink-faint);
	}

	.dot {
		opacity: 0.5;
	}

	.enc {
		display: inline-flex;
		align-items: center;
		gap: 0.25rem;
	}

	.file-list {
		display: flex;
		flex-direction: column;
		border: 1px solid var(--edge);
		border-radius: var(--radius-md);
		overflow: hidden;
		max-height: 240px;
		overflow-y: auto;
	}

	.frow {
		display: flex;
		align-items: center;
		gap: 0.625rem;
		padding: 0.5rem 0.75rem;
		border-bottom: 1px solid var(--edge);

		&:last-child {
			border-bottom: 0;
		}
	}

	.fglyph,
	.flock {
		flex: 0 0 auto;
		display: grid;
		place-items: center;
		color: var(--ink-mute);
	}
	.flock {
		color: var(--ink-faint);
	}

	.fmeta {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
	}

	.fname {
		font-size: var(--fs-sm);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.fsize {
		font-family: var(--font-mono);
		font-size: var(--fs-xs);
		color: var(--ink-faint);
	}

	.fdl {
		flex: 0 0 auto;
		width: 28px;
		height: 28px;
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
			background: var(--tint-softer);
			color: var(--ink);
		}
	}

	.pw-field {
		display: flex;
		flex-direction: column;
		gap: 0.375rem;

		label {
			font-size: var(--fs-xs);
			color: var(--ink-mute);
		}

		input {
			height: 36px;
			padding: 0 0.625rem;
			border-radius: var(--radius-sm);
			background: var(--bg);
			border: 1px solid var(--edge);
			color: var(--ink);
			font-family: var(--font-mono);
			font-size: 0.875rem;
			outline: none;
			transition:
				border-color var(--dur-fast) var(--ease),
				box-shadow var(--dur-fast) var(--ease);

			&::placeholder {
				color: var(--ink-faint);
			}
			&:focus {
				border-color: var(--accent);
				box-shadow: 0 0 0 3px var(--focus-ring);
			}
		}
	}

	.pw-hint {
		font-size: var(--fs-xs);
		color: var(--ink-faint);
	}

	.progress {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}

	.progress-head {
		display: flex;
		align-items: baseline;
		justify-content: space-between;
		font-size: var(--fs-sm);
		color: var(--ink-mute);
	}

	.mono {
		font-family: var(--font-mono);
		font-size: var(--fs-sm);
		color: var(--ink-faint);
	}

	.track {
		height: 6px;
		border-radius: var(--radius-full);
		background: var(--tint-softer);
		overflow: hidden;
	}

	.fill {
		height: 100%;
		border-radius: var(--radius-full);
		background: var(--accent);
		transition: width var(--dur) var(--ease);
	}

	.primary {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 0.4375rem;
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

	.ghost-btn {
		display: inline-flex;
		align-items: center;
		height: 36px;
		padding-inline: 1rem;
		border: 1px solid var(--edge);
		border-radius: var(--radius-md);
		color: var(--ink);
		font-size: var(--fs-sm);
		font-weight: var(--fw-medium);
		text-decoration: none;

		&:hover {
			background: var(--tint-soft);
			color: var(--ink);
		}
	}

	.secure-note {
		margin: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 0.375rem;
		font-size: var(--fs-xs);
		color: var(--ink-faint);
	}

	.report {
		align-self: center;
		display: inline-flex;
		align-items: center;
		gap: 0.25rem;
		border: 0;
		background: none;
		font: inherit;
		font-size: var(--fs-xs);
		color: var(--ink-faint);
		cursor: pointer;
		transition: color var(--dur-fast) var(--ease);

		&:hover:not(:disabled) {
			color: var(--ink-mute);
		}
	}

	.state {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.75rem;
		padding: 2rem 0.5rem;
		text-align: center;
		color: var(--ink-faint);

		h1 {
			margin: 0;
			font-size: var(--fs-lg);
			font-weight: var(--fw-semibold);
			letter-spacing: var(--tracking-tight);
			color: var(--ink);
		}
	}

	.state-line {
		margin: 0;
		font-size: var(--fs-sm);
		color: var(--ink-mute);
		max-width: 34ch;
	}
</style>
