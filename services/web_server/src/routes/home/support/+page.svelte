<script>
	import Icon from '@iconify/svelte';
	import { onMount } from 'svelte';
	import { Button, Badge, EmptyState, Spinner } from '$lib/ui';

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

<div class="view support-page">
	<header class="page-head">
		<div>
			<h1 class="page-title">Support</h1>
			<p class="page-subtitle">Your conversations with the Silocat team.</p>
		</div>
		<Button href="/home/support/new"><Icon icon="ri:add-line" width="16" /> New ticket</Button>
	</header>

	{#if loading}
		<div class="state"><Spinner size={26} /></div>
	{:else if tickets.length === 0}
		<EmptyState
			icon="ri:customer-service-2-line"
			title="No tickets yet"
			line="Questions, bugs, or account help, we're one message away."
		>
			<Button href="/home/support/new"><Icon icon="ri:add-line" width="16" /> Create your first ticket</Button>
		</EmptyState>
	{:else}
		<div class="ticket-list">
			{#each tickets as t (t.id)}
				<a class="ticket" class:closed={t.status === 'closed'} href="/home/support/{t.id}">
					<div class="t-main">
						<div class="t-top">
							<span class="t-cat">{catLabel(t.category)}</span>
							<Badge tone={t.status === 'open' ? 'ok' : 'neutral'}>
								{t.status === 'open' ? 'Open' : 'Resolved'}
							</Badge>
						</div>
						<h3 class="t-subject">{t.subject}</h3>
						<p class="t-snippet">{t.message}</p>
					</div>
					<div class="t-side">
						<span class="t-date">{fmtDate(t.created_at)}</span>
						<Icon icon="ri:arrow-right-s-line" width="18" />
					</div>
				</a>
			{/each}
		</div>
	{/if}
</div>

<style lang="scss">
	.state {
		display: flex;
		justify-content: center;
		padding: var(--space-10) 0;
		color: var(--ink-faint);
	}
	.ticket-list {
		display: flex;
		flex-direction: column;
		border: 1px solid var(--edge);
		border-radius: var(--radius-md);
		overflow: hidden;
	}
	.ticket {
		display: flex;
		align-items: center;
		gap: var(--space-4);
		background: var(--surface);
		padding: var(--space-4) var(--space-5);
		color: inherit;
		text-decoration: none;
		transition: background var(--dur) var(--ease);

		& + .ticket {
			border-top: 1px solid var(--edge);
		}
		&:hover {
			background: var(--surface-hover);
		}
		&.closed {
			opacity: 0.6;
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
		color: var(--ink-faint);
		text-transform: uppercase;
		letter-spacing: 0.04em;
		font-family: var(--font-mono);
		font-size: var(--fs-xs);
	}
	.t-subject {
		font-size: var(--fs-sm);
		font-weight: var(--fw-semibold);
		margin: 0 0 2px;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.t-snippet {
		margin: 0;
		font-size: var(--fs-sm);
		color: var(--ink-faint);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.t-side {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		color: var(--ink-faint);
		flex-shrink: 0;
		.t-date {
			font-size: var(--fs-xs);
			font-family: var(--font-mono);
		}
	}
</style>
