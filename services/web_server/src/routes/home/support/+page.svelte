<script>
	import Icon from '@iconify/svelte';
	import { onMount } from 'svelte';

	const categories = {
		help: 'Need help',
		suggestion: 'Suggestion',
		bug: 'Report a bug',
		account: 'Account / billing',
		other: 'Something else'
	};
	const catLabel = (id) => categories[id] || id;

	let tickets = $state([]);
	let loading = $state(true);

	function fmtDate(iso) {
		try {
			return new Date(iso).toLocaleDateString(undefined, { month: 'short', day: 'numeric', year: 'numeric' });
		} catch {
			return '';
		}
	}

	async function load() {
		loading = true;
		try {
			const res = await fetch('/api/v1/user/support');
			const d = await res.json();
			tickets = d?.success?.data?.tickets ?? [];
		} catch (e) {
			console.error('Failed to load tickets', e);
		} finally {
			loading = false;
		}
	}

	onMount(load);
</script>

<div class="support-page">
	<header class="page-header">
		<div class="title-group">
			<h1>Support</h1>
			<p class="subtitle">Your conversations with the SiloCat team.</p>
		</div>
		<a class="new-btn" href="/home/support/new">
			<Icon icon="ri:add-line" width="18" /> New ticket
		</a>
	</header>

	{#if loading}
		<div class="state"><Icon icon="svg-spinners:ring-resize" width="28" /></div>
	{:else if tickets.length === 0}
		<div class="state empty">
			<Icon icon="ri:customer-service-2-line" width="48" />
			<p>You don't have any tickets yet.</p>
			<a class="new-btn" href="/home/support/new"><Icon icon="ri:add-line" width="18" /> Create your first ticket</a>
		</div>
	{:else}
		<div class="ticket-list">
			{#each tickets as t (t.id)}
				<a class="ticket" class:closed={t.status === 'closed'} href="/home/support/{t.id}">
					<div class="t-main">
						<div class="t-top">
							<span class="t-cat">{catLabel(t.category)}</span>
							<span class="t-status {t.status === 'open' ? 'open' : 'done'}">
								{t.status === 'open' ? 'Open' : 'Resolved'}
							</span>
						</div>
						<h3 class="t-subject">{t.subject}</h3>
						<p class="t-snippet">{t.message}</p>
					</div>
					<div class="t-side">
						<span class="t-date">{fmtDate(t.created_at)}</span>
						<Icon icon="ri:arrow-right-s-line" width="20" />
					</div>
				</a>
			{/each}
		</div>
	{/if}
</div>

<style lang="scss">
	.support-page {
		width: 100%;
		max-width: 820px;
		color: var(--text-primary);
		display: flex;
		flex-direction: column;
		gap: var(--space-6);
	}
	.page-header {
		display: flex;
		justify-content: space-between;
		align-items: flex-end;
		gap: var(--space-4);
		flex-wrap: wrap;
		h1 {
			font-size: var(--fs-h3);
			font-weight: var(--fw-semibold);
			margin: 0 0 var(--space-1);
		}
		.subtitle {
			color: var(--text-muted);
			font-size: var(--fs-sm);
			margin: 0;
		}
	}
	.new-btn {
		display: inline-flex;
		align-items: center;
		gap: var(--space-1);
		background: var(--accent-gradient);
		color: #fff;
		border-radius: var(--radius-pill, 999px);
		padding: var(--space-2) var(--space-4);
		font-weight: var(--fw-semibold);
		font-size: var(--fs-sm);
		box-shadow: 0 6px 20px -6px var(--primary-glow);
		&:hover {
			filter: brightness(1.06);
		}
	}
	.state {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: var(--space-3);
		padding: var(--space-9) 0;
		color: var(--text-muted);
		&.empty p {
			margin: 0;
			font-size: var(--fs-sm);
		}
	}
	.ticket-list {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
	}
	.ticket {
		display: flex;
		align-items: center;
		gap: var(--space-4);
		background: var(--bg-card);
		border: 1px solid var(--border-default);
		border-radius: var(--radius-md);
		box-shadow: var(--shadow-card);
		padding: var(--space-4) var(--space-5);
		color: inherit;
		text-decoration: none;
		transition: border-color var(--dur) var(--ease), background var(--dur) var(--ease);
		&:hover {
			border-color: var(--border-active, var(--primary));
			background: var(--bg-card-hover);
		}
		&.closed {
			opacity: 0.72;
		}
	}
	.t-main {
		flex: 1;
		min-width: 0;
	}
	.t-top {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		margin-bottom: var(--space-1);
	}
	.t-cat {
		color: var(--text-muted);
		text-transform: uppercase;
		letter-spacing: 0.04em;
		font-family: var(--font-mono);
		font-size: var(--fs-xs);
	}
	.t-status {
		font-size: 0.62rem;
		font-weight: var(--fw-semibold);
		text-transform: uppercase;
		letter-spacing: 0.04em;
		padding: 2px 8px;
		border-radius: 999px;
		&.open {
			background: color-mix(in srgb, var(--success, #3ddc97) 16%, transparent);
			color: var(--success, #3ddc97);
		}
		&.done {
			background: var(--tint-soft);
			color: var(--text-muted);
		}
	}
	.t-subject {
		font-size: var(--fs-body);
		font-weight: var(--fw-semibold);
		margin: 0 0 2px;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.t-snippet {
		margin: 0;
		font-size: var(--fs-sm);
		color: var(--text-muted);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.t-side {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		color: var(--text-dim);
		flex-shrink: 0;
		.t-date {
			font-size: var(--fs-xs);
			font-family: var(--font-mono);
		}
	}
</style>
