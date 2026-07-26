<script>
	import Icon from '$lib/ui/Icon.svelte';
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
	class="pv-scrim"
	transition:fade={{ duration: 150 }}
	role="presentation"
	onclick={(e) => {
		if (e.target === e.currentTarget) onclose();
	}}
>
	<div class="pv-shell" transition:scale={{ duration: 190, start: 0.96 }}>
		<header class="pv-head">
			<span class="pv-chip"><Icon name="eye" size={16} /></span>
			<span class="pv-title" title={file?.name}>{file?.name}</span>
			<button type="button" class="pv-btn" onclick={() => ondownload()} title="Download" aria-label="Download">
				<Icon name="download" size={15} />
			</button>
			<button type="button" class="pv-btn" onclick={() => onclose()} aria-label="Close">
				<Icon name="close" size={15} />
			</button>
		</header>

		<div class="pv-body">
			{#if status === 'loading'}
				<div class="pv-state">
					<Icon name="spinner" size={26} />
					<span class="pv-state-title">Decrypting…</span>
					<span class="pv-state-line">{progress}%: this happens in your browser.</span>
				</div>
			{:else if status === 'error'}
				<div class="pv-state">
					<Icon name="alert" size={30} stroke={1.4} />
					<span class="pv-state-title">Could not open this file</span>
					<span class="pv-state-line">{error}</span>
				</div>
			{:else if status === 'unsupported'}
				<div class="pv-state">
					<Icon name="file" size={32} stroke={1.3} />
					<span class="pv-state-title">No inline preview</span>
					<span class="pv-state-line">This file type can't be shown here, but it downloads fine.</span>
					<button type="button" class="pv-cta" onclick={() => ondownload()}>
						<Icon name="download" size={15} /> Download instead
					</button>
				</div>
			{:else if kind === 'image'}
				<img src={url} alt={file?.name} />
			{:else if kind === 'video'}
				<!-- svelte-ignore a11y_media_has_caption -->
				<video src={url} controls autoplay></video>
			{:else if kind === 'audio'}
				<div class="pv-audio">
					<Icon name="audio" size={40} stroke={1.3} />
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
	.pv-scrim {
		position: fixed;
		inset: 0;
		z-index: 1000;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: var(--space-5);
		background: var(--scrim);
	}

	.pv-shell {
		width: 100%;
		max-width: 960px;
		max-height: 88vh;
		display: flex;
		flex-direction: column;
		background: var(--raised);
		border: 1px solid var(--edge);
		border-radius: var(--radius-lg);
		box-shadow: var(--shadow-overlay);
		overflow: hidden;
	}

	.pv-head {
		display: flex;
		align-items: center;
		gap: 0.625rem;
		padding: 1rem 1rem 0.875rem;
		flex: 0 0 auto;
	}

	.pv-chip {
		display: grid;
		place-items: center;
		width: 28px;
		height: 28px;
		border-radius: 8px;
		background: var(--tint-soft);
		color: var(--ink-mute);
		flex: 0 0 auto;
	}

	.pv-title {
		flex: 1;
		min-width: 0;
		font-size: 0.9375rem;
		font-weight: var(--fw-semibold);
		letter-spacing: var(--tracking-tight);
		color: var(--ink);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.pv-btn {
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
			background: var(--tint-soft);
			color: var(--ink);
		}
	}

	/* The media sits on --bg, a shade below the panel, so a transparent PNG or a
	   letterboxed video reads as content rather than a hole in the dialog. */
	.pv-body {
		flex: 1;
		min-height: 0;
		margin: 0 1rem 1rem;
		border: 1px solid var(--edge);
		border-radius: var(--radius-md);
		background: var(--bg);
		overflow: auto;
		display: flex;
		align-items: center;
		justify-content: center;

		img,
		video {
			max-width: 100%;
			max-height: 70vh;
			object-fit: contain;
			display: block;
		}

		iframe {
			width: 100%;
			height: 70vh;
			border: 0;
			background: #fff;
		}

		pre {
			margin: 0;
			padding: 1rem;
			width: 100%;
			align-self: stretch;
			font-family: var(--font-mono);
			font-size: var(--fs-sm);
			line-height: var(--lh-normal);
			color: var(--ink-mute);
			white-space: pre-wrap;
			word-break: break-word;
		}
	}

	.pv-audio {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: var(--space-4);
		padding: 3rem 1.5rem;
		color: var(--ink-faint);

		audio {
			width: min(420px, 70vw);
		}
	}

	.pv-state {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.625rem;
		padding: 4rem 1.5rem;
		text-align: center;
		color: var(--ink-faint);
	}

	.pv-state-title {
		font-size: var(--fs-lg);
		font-weight: var(--fw-medium);
		letter-spacing: var(--tracking-tight);
		color: var(--ink);
	}

	.pv-state-line {
		font-size: var(--fs-sm);
		color: var(--ink-mute);
		max-width: 40ch;
	}

	.pv-cta {
		display: inline-flex;
		align-items: center;
		gap: 0.4375rem;
		margin-top: var(--space-2);
		height: 34px;
		padding-inline: 0.875rem;
		border: 1px solid var(--edge);
		border-radius: var(--radius-md);
		background: none;
		font: inherit;
		font-size: var(--fs-sm);
		font-weight: var(--fw-medium);
		color: var(--ink);
		cursor: pointer;
		transition:
			background var(--dur-fast) var(--ease),
			border-color var(--dur-fast) var(--ease);

		&:hover {
			background: var(--tint-soft);
			border-color: var(--edge-strong);
		}
	}

	@media (max-width: 640px) {
		.pv-scrim {
			padding: 0;
			align-items: flex-end;
		}
		.pv-shell {
			max-width: none;
			max-height: 92vh;
			border-radius: var(--radius-lg) var(--radius-lg) 0 0;
		}
	}
</style>
