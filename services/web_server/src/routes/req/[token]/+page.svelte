<script>
	import { onMount } from 'svelte';
	import { formatSize } from '$lib/format.js';
	import { page } from '$app/stores';
	import axios from 'axios';
	import { uploadToRequest } from '$lib/requestUpload.js';
	import { holdTransfer, guardNavigation } from '$lib/transferGuard.js';
	import { generatePassword } from '$lib/password.js';

	const token = $page.params.token;

	guardNavigation('Your file is still uploading. Leaving now will cancel it.');

	let loading = $state(true);
	let info = $state(null); // { label, message, open }
	let error = $state('');

	let file = $state(null);
	let uploaderName = $state('');
	let encrypt = $state(false);
	let password = $state('');

	let uploading = $state(false);
	let progress = $state(0);
	let done = $state(false);

	onMount(async () => {
		try {
			const res = await axios.get(`/api/v1/public/request/info/${token}`);
			info = res.data?.success?.data ?? res.data?.data ?? null;
			if (!info) error = 'This request link is invalid.';
		} catch (e) {
			error = e.response?.data?.message || 'This request link is invalid or has expired.';
		} finally {
			loading = false;
		}
	});

	// Matches the anonymous ceiling enforced in the shadow upload route.
	const MAX_UPLOAD_BYTES = 20 * 1024 * 1024 * 1024;

	let isDragging = $state(false);


	/** Accept a file, refusing oversize ones here rather than after the upload. */
	function accept(candidate) {
		if (!candidate) return;
		if (candidate.size > MAX_UPLOAD_BYTES) {
			error = `That file is ${formatSize(candidate.size)}. The limit is ${formatSize(MAX_UPLOAD_BYTES)}.`;
			return;
		}
		error = '';
		file = candidate;
	}

	function pick(e) {
		accept(e.target.files?.[0] || null);
	}

	function onDrop(e) {
		e.preventDefault();
		isDragging = false;
		accept(e.dataTransfer?.files?.[0] || null);
	}

	// The rest of the app offers a generated password wherever one is required;
	// this page asked a stranger to invent one with no help.
	function makePassword() {
		password = generatePassword();
	}

	async function submit() {
		if (!file || uploading) return;
		if (encrypt && !password) {
			error = 'Set a password to encrypt, or turn encryption off.';
			return;
		}
		error = '';
		uploading = true;
		progress = 0;
		const releaseTransfer = holdTransfer();
		try {
			await uploadToRequest(token, file, {
				encrypt,
				password,
				uploaderName,
				onProgress: (p) => (progress = p)
			});
			done = true;
		} catch (e) {
			error = e.response?.data?.message || e.message || 'Upload failed. Please try again.';
		} finally {
			uploading = false;
			releaseTransfer();
		}
	}
</script>

<svelte:head>
	<title>Send a file securely · Silocat</title>
	<meta name="robots" content="noindex" />
</svelte:head>

<main class="wrap">
	<div class="card">
		{#if loading}
			<p class="muted">Loading…</p>
		{:else if error && !info}
			<h1>Link unavailable</h1>
			<p class="muted">{error}</p>
		{:else if done}
			<div class="check">✓</div>
			<h1>Sent</h1>
			<p class="muted">Your file was delivered securely. You can close this page.</p>
		{:else}
			<p class="eyebrow">Someone asked you to send a file</p>
			<h1>{info?.label || 'Send a file'}</h1>
			{#if info?.message}<p class="message">{info.message}</p>{/if}

			{#if info && info.open === false}
				<p class="muted">This request is closed and is no longer accepting uploads.</p>
			{:else}
				<label class="field">
					<span>Your name (optional)</span>
					<input type="text" bind:value={uploaderName} placeholder="So they know who it's from" />
				</label>

				<!-- Drag and drop, like every other upload surface in the product. This
				     page is the first thing an outside recipient sees of Silocat and it
				     was a bare file input. -->
				<label
					class="drop"
					class:dragging={isDragging}
					ondragover={(e) => {
						e.preventDefault();
						isDragging = true;
					}}
					ondragleave={() => (isDragging = false)}
					ondrop={onDrop}
				>
					<input type="file" onchange={pick} />
					{#if file}
						<span class="drop-name">{file.name}</span>
						<span class="drop-sub">{formatSize(file.size)}: click to choose a different file</span>
					{:else}
						<span class="drop-name">Drop a file here, or click to choose</span>
						<span class="drop-sub">Up to {formatSize(MAX_UPLOAD_BYTES)}</span>
					{/if}
				</label>

				<label class="check-row">
					<input type="checkbox" bind:checked={encrypt} />
					<span>Encrypt end to end (they'll need the password below to open it)</span>
				</label>
				{#if encrypt}
					<div class="field">
						<span>Password</span>
						<div class="pw-row">
							<input
								type="text"
								bind:value={password}
								placeholder="Share this with the recipient"
								autocomplete="off"
							/>
							<button type="button" class="gen" onclick={makePassword}>Generate</button>
						</div>
						<!-- The sender has to get this to the recipient themselves, and
						     nothing on the page used to say so. Losing it means the file is
						     unrecoverable, which is worth one sentence. -->
						<span class="hint">
							Send this to them separately, not in the same message as the link. Nobody,
							including us, can recover the file without it.
						</span>
					</div>
				{/if}

				{#if error}<p class="err">{error}</p>{/if}

				{#if uploading}
					<div class="bar"><div class="fill" style="width:{progress}%"></div></div>
					<p class="muted">Uploading… {progress}%</p>
				{:else}
					<button class="send" onclick={submit} disabled={!file}>Send securely</button>
				{/if}

				<p class="foot">
					Powered by Silocat ·
					{encrypt
						? 'encrypted in this browser before it is sent'
						: 'encrypted in transit and at rest'}
				</p>
			{/if}
		{/if}
	</div>
</main>

<style>
	.wrap {
		min-height: 100dvh;
		display: grid;
		place-items: center;
		padding: 2rem 1rem;
		background: var(--bg-base, #0b0b0d);
		color: var(--ink, #e8e8ea);
	}
	.card {
		width: 100%;
		max-width: 440px;
		background: var(--bg-elev, #16161a);
		border: 1px solid var(--border, #26262b);
		border-radius: 14px;
		padding: 1.75rem;
	}
	.eyebrow {
		font-size: 0.72rem;
		letter-spacing: 0.08em;
		text-transform: uppercase;
		color: var(--ink-faint, #8a8a92);
		margin: 0 0 0.35rem;
	}
	h1 {
		font-size: 1.4rem;
		margin: 0 0 0.5rem;
	}
	.message {
		color: var(--ink-mute, #b6b6bd);
		margin: 0 0 1rem;
		white-space: pre-wrap;
	}
	.muted {
		color: var(--ink-faint, #8a8a92);
	}
	.field {
		display: block;
		margin: 0.9rem 0;
	}
	.field span {
		display: block;
		font-size: 0.78rem;
		color: var(--ink-mute, #b6b6bd);
		margin-bottom: 0.3rem;
	}
	.field input {
		width: 100%;
		padding: 0.6rem 0.7rem;
		background: var(--bg-base, #0b0b0d);
		border: 1px solid var(--border, #26262b);
		border-radius: 8px;
		color: var(--ink, #e8e8ea);
	}
	.drop {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
		border: 1px dashed var(--border, #33333a);
		border-radius: 10px;
		padding: 1.1rem;
		text-align: center;
		cursor: pointer;
		margin: 0.9rem 0;
		color: var(--ink-mute, #b6b6bd);
		transition: border-color 0.15s ease, background 0.15s ease;
	}
	.drop:hover,
	.drop.dragging {
		border-color: var(--accent, #6ea8fe);
		background: rgba(110, 168, 254, 0.06);
	}
	.drop input {
		display: none;
	}
	.drop-name {
		font-size: 0.9rem;
		color: var(--ink, #e8e8ea);
		word-break: break-word;
	}
	.drop-sub {
		font-size: 0.75rem;
		color: var(--ink-mute, #b6b6bd);
	}
	.pw-row {
		display: flex;
		gap: 0.4rem;
	}
	.pw-row input {
		flex: 1;
		min-width: 0;
	}
	.gen {
		flex: 0 0 auto;
		padding: 0 0.7rem;
		border-radius: 8px;
		border: 1px solid var(--border, #33333a);
		background: transparent;
		color: var(--ink, #e8e8ea);
		font-size: 0.78rem;
		cursor: pointer;
	}
	.gen:hover {
		border-color: var(--accent, #6ea8fe);
		color: var(--accent, #6ea8fe);
	}
	.hint {
		display: block;
		margin-top: 0.35rem;
		font-size: 0.72rem;
		line-height: 1.45;
		color: var(--ink-mute, #b6b6bd);
	}
	.check-row {
		display: flex;
		gap: 0.5rem;
		align-items: flex-start;
		font-size: 0.82rem;
		color: var(--ink-mute, #b6b6bd);
		margin: 0.6rem 0;
	}
	.send {
		width: 100%;
		padding: 0.75rem;
		border: none;
		border-radius: 9px;
		background: var(--accent, #ff4655);
		color: #fff;
		font-weight: 600;
		cursor: pointer;
		margin-top: 0.6rem;
	}
	.send:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}
	.bar {
		height: 8px;
		background: var(--bg-base, #0b0b0d);
		border-radius: 6px;
		overflow: hidden;
		margin: 0.8rem 0 0.3rem;
	}
	.fill {
		height: 100%;
		background: var(--accent, #ff4655);
		transition: width 0.2s ease;
	}
	.err {
		color: var(--danger, #ff6b6b);
		font-size: 0.82rem;
	}
	.check {
		font-size: 2.5rem;
		color: var(--accent, #ff4655);
	}
	.foot {
		font-size: 0.72rem;
		color: var(--ink-faint, #8a8a92);
		margin: 1rem 0 0;
		text-align: center;
	}
</style>
