<script>
	import Icon from '@iconify/svelte';
	import { onMount, onDestroy } from 'svelte';
	import { fade, scale } from 'svelte/transition';
	import { fetchDecryptedBlob } from '$lib/download.js';

	// file: { id, name, mime, size, encrypted, type }
	let { file, password = null, onclose = () => {}, ondownload = () => {} } = $props();

	let status = $state('loading'); // 'loading' | 'ready' | 'error' | 'unsupported'
	let url = $state(null);
	let error = $state('');
	let progress = $state(0);
	let kind = $state('other'); // 'image' | 'video' | 'audio' | 'pdf' | 'text' | 'other'
	let textContent = $state('');

	let controller = new AbortController();

	function resolveKind(mime = '') {
		if (mime.startsWith('image/')) return 'image';
		if (mime.startsWith('video/')) return 'video';
		if (mime.startsWith('audio/')) return 'audio';
		if (mime === 'application/pdf') return 'pdf';
		if (mime.startsWith('text/') || mime === 'application/json') return 'text';
		return 'other';
	}

	onMount(async () => {
		kind = resolveKind(file?.mime || '');
		if (kind === 'other') {
			status = 'unsupported';
			return;
		}
		try {
			const blob = await fetchDecryptedBlob(file, {
				password,
				signal: controller.signal,
				onProgress: (loaded, total) => {
					progress = total ? Math.round((loaded / total) * 100) : 0;
				}
			});
			url = URL.createObjectURL(blob);
			if (kind === 'text') textContent = await blob.text();
			status = 'ready';
		} catch (e) {
			if (controller.signal.aborted) return;
			console.error('[preview]', e);
			error = e?.message || 'Could not load preview';
			status = 'error';
		}
	});

	onDestroy(() => {
		controller.abort();
		if (url) URL.revokeObjectURL(url);
	});
</script>

<div
	class="preview-backdrop"
	transition:fade={{ duration: 150 }}
	role="presentation"
	onclick={(e) => {
		if (e.target === e.currentTarget) onclose();
	}}
>
	<div class="preview-shell" transition:scale={{ duration: 180, start: 0.97 }}>
		<header class="preview-head">
			<div class="title" title={file?.name}>
				<Icon icon="ri:eye-line" width="18" />
				<span>{file?.name}</span>
			</div>
			<div class="head-actions">
				<button class="hbtn" onclick={() => ondownload()} title="Download">
					<Icon icon="ri:download-line" width="18" />
				</button>
				<button class="hbtn" onclick={() => onclose()} aria-label="Close">
					<Icon icon="ri:close-line" width="20" />
				</button>
			</div>
		</header>

		<div class="preview-body">
			{#if status === 'loading'}
				<div class="state">
					<Icon icon="ri:loader-4-line" class="spin" width="32" />
					<p>Decrypting and loading… {progress}%</p>
				</div>
			{:else if status === 'error'}
				<div class="state">
					<Icon icon="ri:error-warning-line" width="32" />
					<p>{error}</p>
				</div>
			{:else if status === 'unsupported'}
				<div class="state">
					<Icon icon="ri:file-3-line" width="40" />
					<p>No inline preview for this file type.</p>
					<button class="dl-btn" onclick={() => ondownload()}>
						<Icon icon="ri:download-line" width="16" /> Download instead
					</button>
				</div>
			{:else if kind === 'image'}
				<img src={url} alt={file?.name} />
			{:else if kind === 'video'}
				<!-- svelte-ignore a11y_media_has_caption -->
				<video src={url} controls autoplay></video>
			{:else if kind === 'audio'}
				<div class="audio-wrap">
					<Icon icon="ri:music-2-line" width="48" />
					<audio src={url} controls autoplay></audio>
				</div>
			{:else if kind === 'pdf'}
				<iframe src={url} title={file?.name}></iframe>
			{:else if kind === 'text'}
				<pre>{textContent}</pre>
			{/if}
		</div>
	</div>
</div>

<style lang="scss">
	.preview-backdrop {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.8);
		backdrop-filter: blur(10px);
		z-index: 1100;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: var(--space-5);
	}
	.preview-shell {
		background: var(--bg-elevated);
		border: 1px solid var(--border-default);
		border-radius: var(--radius-lg);
		box-shadow: var(--shadow-lg);
		width: 100%;
		max-width: 920px;
		max-height: 92vh;
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}
	.preview-head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-3);
		padding: var(--space-3) var(--space-4);
		border-bottom: 1px solid var(--hairline);

		.title {
			display: flex;
			align-items: center;
			gap: var(--space-2);
			min-width: 0;
			color: var(--text-primary);
			font-weight: var(--fw-medium);
			span {
				overflow: hidden;
				text-overflow: ellipsis;
				white-space: nowrap;
			}
		}
		.head-actions {
			display: flex;
			gap: var(--space-1);
			flex: none;
		}
		.hbtn {
			background: transparent;
			border: none;
			color: var(--text-muted);
			cursor: pointer;
			padding: 6px;
			border-radius: var(--radius-sm);
			display: flex;
			transition: color var(--dur) var(--ease), background var(--dur) var(--ease);
			&:hover {
				color: var(--text-primary);
				background: var(--tint-soft);
			}
		}
	}
	.preview-body {
		flex: 1;
		min-height: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		background: var(--bg-base, #0a0a0c);
		overflow: auto;

		img,
		video {
			max-width: 100%;
			max-height: 86vh;
			display: block;
			object-fit: contain;
		}
		iframe {
			width: 100%;
			height: 86vh;
			border: none;
			background: #fff;
		}
		pre {
			margin: 0;
			padding: var(--space-5);
			width: 100%;
			height: 100%;
			max-height: 86vh;
			overflow: auto;
			color: var(--text-secondary);
			font-family: var(--font-mono);
			font-size: var(--fs-sm);
			white-space: pre-wrap;
			word-break: break-word;
		}
		.audio-wrap {
			display: flex;
			flex-direction: column;
			align-items: center;
			gap: var(--space-4);
			color: var(--text-secondary);
			padding: var(--space-6);
		}
		.state {
			display: flex;
			flex-direction: column;
			align-items: center;
			gap: var(--space-3);
			color: var(--text-muted);
			padding: var(--space-7) var(--space-5);
			text-align: center;
			:global(.spin) {
				animation: spin 1s linear infinite;
				color: var(--primary);
			}
			.dl-btn {
				display: inline-flex;
				align-items: center;
				gap: var(--space-2);
				background: var(--primary);
				color: #fff;
				border: none;
				border-radius: var(--radius-sm);
				padding: var(--space-2) var(--space-4);
				cursor: pointer;
				font-weight: var(--fw-medium);
			}
		}
	}
	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
	}
	@media (max-width: 600px) {
		.preview-backdrop {
			padding: 0;
		}
		.preview-shell {
			max-width: 100%;
			max-height: 100%;
			height: 100%;
			border-radius: 0;
		}
	}
</style>
