<script>
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import axios from 'axios';
	import { uploadToRequest } from '$lib/requestUpload.js';

	const token = $page.params.token;

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

	function pick(e) {
		file = e.target.files?.[0] || null;
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

				<label class="drop">
					<input type="file" onchange={pick} />
					<span>{file ? file.name : 'Choose a file to send'}</span>
				</label>

				<label class="check-row">
					<input type="checkbox" bind:checked={encrypt} />
					<span>Encrypt end to end (they'll need the password below to open it)</span>
				</label>
				{#if encrypt}
					<label class="field">
						<span>Password</span>
						<input type="password" bind:value={password} placeholder="Share this with the recipient" autocomplete="new-password" />
					</label>
				{/if}

				{#if error}<p class="err">{error}</p>{/if}

				{#if uploading}
					<div class="bar"><div class="fill" style="width:{progress}%"></div></div>
					<p class="muted">Uploading… {progress}%</p>
				{:else}
					<button class="send" onclick={submit} disabled={!file}>Send securely</button>
				{/if}

				<p class="foot">Powered by Silocat · files are stored encrypted at rest.</p>
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
		display: block;
		border: 1px dashed var(--border, #33333a);
		border-radius: 10px;
		padding: 1.1rem;
		text-align: center;
		cursor: pointer;
		margin: 0.9rem 0;
		color: var(--ink-mute, #b6b6bd);
	}
	.drop input {
		display: none;
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
