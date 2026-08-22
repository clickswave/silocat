<script>
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import axios from 'axios';

	// Request-a-file is a paid delivery feature (E7): a subscription unlocks it.
	let isPaid = $derived(!!$page.data.user?.subscription);

	let requests = $state([]);
	let loading = $state(true);
	let creating = $state(false);

	let label = $state('');
	let message = $state('');
	let expiresDays = $state(0);
	let maxUploads = $state(0);

	let received = $state({}); // request id -> files[] (or undefined when collapsed)
	let copied = $state('');

	async function load() {
		loading = true;
		try {
			const res = await axios.get('/api/v1/sanctum/file/request/list');
			requests = res.data?.success?.data?.requests ?? [];
		} catch (e) {
			// leave the list as-is on a transient error
		} finally {
			loading = false;
		}
	}
	onMount(load);

	async function create() {
		if (creating) return;
		creating = true;
		try {
			const body = { label: label || null, message: message || null };
			if (Number(expiresDays) > 0) body.expires_in_days = Number(expiresDays);
			if (Number(maxUploads) > 0) body.max_uploads = Number(maxUploads);
			await axios.post('/api/v1/sanctum/file/request/create', body);
			label = '';
			message = '';
			expiresDays = 0;
			maxUploads = 0;
			await load();
		} catch (e) {
			// ignore; the form stays filled for a retry
		} finally {
			creating = false;
		}
	}

	function linkFor(token) {
		return `${window.location.origin}/req/${token}`;
	}
	function copy(token) {
		navigator.clipboard.writeText(linkFor(token));
		copied = token;
		setTimeout(() => (copied = ''), 1500);
	}

	async function toggle(r) {
		await axios.post('/api/v1/sanctum/file/request/set-active', { id: r.id, active: !r.active });
		await load();
	}

	async function toggleReceived(r) {
		if (received[r.id]) {
			received = { ...received, [r.id]: undefined };
			return;
		}
		const res = await axios.post('/api/v1/sanctum/file/request/received', { id: r.id });
		received = { ...received, [r.id]: res.data?.success?.data?.files ?? [] };
	}
</script>

<svelte:head><title>Requests · Silocat</title></svelte:head>

<section class="requests">
	<header>
		<h1>Request a file</h1>
		<p class="sub">
			Create a link and send it to anyone. Whatever they upload lands in your account, no
			account needed on their end. The inbound half of secure delivery.
		</p>
	</header>

	{#if !isPaid}
		<div class="upgrade">
			<h2>Requesting files is a paid feature</h2>
			<p>
				Create a link and let clients, colleagues, or anyone send files straight into your
				account, no account needed on their end. Upgrade to turn it on.
			</p>
			<a class="upgrade-btn" href="/home/billing">See plans</a>
		</div>
	{:else}
	<div class="new">
		<div class="row">
			<input class="grow" type="text" bind:value={label} placeholder="What are you asking for? (e.g. Signed contract)" />
		</div>
		<div class="row">
			<input class="grow" type="text" bind:value={message} placeholder="A note to the sender (optional)" />
		</div>
		<div class="row small">
			<label>Expires in
				<input type="number" min="0" bind:value={expiresDays} /> days (0 = never)
			</label>
			<label>Max uploads
				<input type="number" min="0" bind:value={maxUploads} /> (0 = unlimited)
			</label>
			<button class="create" onclick={create} disabled={creating}>
				{creating ? 'Creating…' : 'Create request'}
			</button>
		</div>
	</div>

	{#if loading}
		<p class="muted">Loading…</p>
	{:else if requests.length === 0}
		<p class="muted">No requests yet. Create one above and share the link.</p>
	{:else}
		<ul class="list">
			{#each requests as r (r.id)}
				<li class="item" class:closed={!r.active}>
					<div class="top">
						<div class="meta">
							<strong>{r.label || 'Untitled request'}</strong>
							<span class="count">{r.upload_count} received{r.max_uploads ? ` / ${r.max_uploads}` : ''}</span>
							{#if !r.active}<span class="tag">Closed</span>{/if}
						</div>
						<div class="acts">
							<button onclick={() => copy(r.token)}>{copied === r.token ? 'Copied' : 'Copy link'}</button>
							<button onclick={() => toggleReceived(r)}>
								{received[r.id] ? 'Hide files' : 'View files'}
							</button>
							<button onclick={() => toggle(r)}>{r.active ? 'Close' : 'Reopen'}</button>
						</div>
					</div>
					<code class="link">{linkFor(r.token)}</code>
					{#if received[r.id]}
						{#if received[r.id].length === 0}
							<p class="muted small-note">Nothing received yet.</p>
						{:else}
							<ul class="files">
								{#each received[r.id] as f (f.id)}
									<li>
										<span class="fname">{f.name}</span>
										<span class="fsize">{(Number(f.size) / 1024 / 1024).toFixed(2)} MB</span>
									</li>
								{/each}
							</ul>
						{/if}
					{/if}
				</li>
			{/each}
		</ul>
	{/if}
	{/if}
</section>

<style>
	.requests {
		max-width: 760px;
		padding: 1.5rem;
	}
	h1 {
		margin: 0 0 0.3rem;
	}
	.sub {
		color: var(--ink-mute, #b6b6bd);
		margin: 0 0 1.25rem;
		max-width: 60ch;
	}
	.muted {
		color: var(--ink-faint, #8a8a92);
	}
	.upgrade {
		background: var(--bg-elev, #16161a);
		border: 1px solid var(--border, #26262b);
		border-radius: 12px;
		padding: 1.5rem;
		max-width: 520px;
	}
	.upgrade h2 {
		margin: 0 0 0.4rem;
		font-size: 1.1rem;
	}
	.upgrade p {
		color: var(--ink-mute, #b6b6bd);
		margin: 0 0 1rem;
	}
	.upgrade-btn {
		display: inline-block;
		padding: 0.55rem 1rem;
		background: var(--accent, #ff4655);
		color: #fff;
		border-radius: 8px;
		text-decoration: none;
		font-weight: 600;
	}
	.new {
		background: var(--bg-elev, #16161a);
		border: 1px solid var(--border, #26262b);
		border-radius: 12px;
		padding: 1rem;
		margin-bottom: 1.5rem;
	}
	.row {
		display: flex;
		gap: 0.6rem;
		margin-bottom: 0.6rem;
		align-items: center;
	}
	.row.small {
		font-size: 0.82rem;
		color: var(--ink-mute, #b6b6bd);
		flex-wrap: wrap;
	}
	.grow {
		flex: 1;
	}
	input {
		padding: 0.55rem 0.65rem;
		background: var(--bg-base, #0b0b0d);
		border: 1px solid var(--border, #26262b);
		border-radius: 8px;
		color: var(--ink, #e8e8ea);
	}
	.row.small input {
		width: 4.5rem;
		margin: 0 0.3rem;
	}
	button {
		padding: 0.5rem 0.75rem;
		border: 1px solid var(--border, #33333a);
		background: var(--bg-base, #0b0b0d);
		color: var(--ink, #e8e8ea);
		border-radius: 8px;
		cursor: pointer;
	}
	.create {
		background: var(--accent, #ff4655);
		color: #fff;
		border: none;
		margin-left: auto;
	}
	.list {
		list-style: none;
		margin: 0;
		padding: 0;
		display: grid;
		gap: 0.75rem;
	}
	.item {
		border: 1px solid var(--border, #26262b);
		border-radius: 12px;
		padding: 0.9rem 1rem;
		background: var(--bg-elev, #16161a);
	}
	.item.closed {
		opacity: 0.6;
	}
	.top {
		display: flex;
		justify-content: space-between;
		gap: 0.75rem;
		flex-wrap: wrap;
	}
	.meta {
		display: flex;
		align-items: center;
		gap: 0.6rem;
		flex-wrap: wrap;
	}
	.count {
		font-size: 0.78rem;
		color: var(--ink-faint, #8a8a92);
	}
	.tag {
		font-size: 0.7rem;
		border: 1px solid var(--border, #33333a);
		border-radius: 999px;
		padding: 1px 8px;
		color: var(--ink-faint, #8a8a92);
	}
	.acts {
		display: flex;
		gap: 0.4rem;
	}
	.link {
		display: block;
		margin-top: 0.5rem;
		font-size: 0.76rem;
		color: var(--ink-faint, #8a8a92);
		word-break: break-all;
	}
	.files {
		list-style: none;
		margin: 0.6rem 0 0;
		padding: 0.6rem 0 0;
		border-top: 1px solid var(--border, #26262b);
	}
	.files li {
		display: flex;
		justify-content: space-between;
		padding: 3px 0;
		font-size: 0.85rem;
	}
	.fsize {
		color: var(--ink-faint, #8a8a92);
	}
	.small-note {
		margin: 0.5rem 0 0;
		font-size: 0.82rem;
	}
</style>
