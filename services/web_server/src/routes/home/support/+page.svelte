<script>
	import Icon from '$lib/ui/Icon.svelte';
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

	onMount(load);</script>

<div class="support">
	<header class="head">
		<div class="head-text">
			<h1>Support</h1>
			<span class="sub">Your conversations with the Silocat team.</span>
		</div>
		<a href="/home/support/new" class="new-btn">
			<Icon name="plus" size={15} />
			New ticket
		</a>
	</header>

	{#if loading}
		<div class="state"><Icon name="spinner" size={26} /></div>
	{:else if tickets.length === 0}
		<div class="empty">
			<Icon name="support" size={34} stroke={1.2} />
			<div class="empty-text">
				<span class="empty-title">No tickets yet</span>
				<span class="empty-line">Questions, bugs, or account help, we're one message away.</span>
			</div>
			<a href="/home/support/new" class="new-btn">
				<Icon name="plus" size={15} />
				Create your first ticket
			</a>
		</div>
	{:else}
		<div class="list">
			{#each tickets as t (t.id)}
				<a class="ticket" class:closed={t.status !== 'open'} href="/home/support/{t.id}">
					<div class="t-main">
						<div class="t-top">
							<span class="t-cat">{catLabel(t.category)}</span>
							<span class="t-status {t.status === 'open' ? 'open' : 'done'}">
								{t.status === 'open' ? 'Open' : 'Resolved'}
							</span>
						</div>
						<span class="t-subject">{t.subject}</span>
						<span class="t-snippet">{t.message}</span>
					</div>
					<div class="t-side">
						<span class="t-date">{fmtDate(t.created_at)}</span>
						<Icon name="chevron-right" size={16} />
					</div>
				</a>
			{/each}
		</div>
	{/if}
</div>

<style lang="scss">
	.support {
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
		padding-bottom: var(--space-6);
	}

	.head {
		display: flex;
		align-items: flex-end;
		justify-content: space-between;
		gap: var(--space-4);
		padding: var(--space-2) 0.125rem 0;
	}

	.head-text {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);

		h1 {
			margin: 0;
			font-size: var(--fs-h2);
			font-weight: var(--fw-black);
			letter-spacing: var(--tracking-tight);
			line-height: var(--lh-tight);
		}
	}

	.sub {
		font-size: var(--fs-sm);
		color: var(--ink-faint);
	}

	.new-btn {
		display: inline-flex;
		align-items: center;
		gap: 0.4375rem;
		height: 34px;
		padding-inline: 0.875rem;
		border-radius: var(--radius-md);
		background: var(--accent);
		color: #fff;
		font-size: var(--fs-sm);
		font-weight: var(--fw-medium);
		text-decoration: none;
		flex: 0 0 auto;
		transition: background var(--dur-fast) var(--ease);

		&:hover {
			background: var(--accent-hover);
			color: #fff;
		}
	}

	.list {
		display: flex;
		flex-direction: column;
		border: 1px solid var(--edge);
		border-radius: var(--radius-md);
		background: var(--surface);
		overflow: hidden;
	}

	.ticket {
		display: flex;
		align-items: center;
		gap: var(--space-4);
		padding: 0.875rem 1rem;
		color: inherit;
		text-decoration: none;
		transition: background var(--dur-fast) var(--ease);

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
		display: flex;
		flex-direction: column;
		gap: 0.125rem;
	}

	.t-top {
		display: flex;
		align-items: center;
		gap: var(--space-2);
	}

	.t-cat {
		font-family: var(--font-mono);
		font-size: 0.6875rem;
		color: var(--ink-faint);
		text-transform: uppercase;
		letter-spacing: 0.08em;
	}

	.t-status {
		display: inline-flex;
		align-items: center;
		height: 18px;
		padding-inline: 0.375rem;
		border-radius: var(--radius-sm);
		font-size: 0.6875rem;
		font-weight: var(--fw-medium);

		&.open {
			background: var(--ok-soft);
			color: var(--ok);
		}
		&.done {
			background: var(--tint-softer);
			color: var(--ink-mute);
		}
	}

	.t-subject {
		font-size: var(--fs-sm);
		font-weight: var(--fw-semibold);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.t-snippet {
		font-size: var(--fs-sm);
		color: var(--ink-faint);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.t-side {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		color: var(--ink-faint);
		flex-shrink: 0;
	}

	.t-date {
		font-family: var(--font-mono);
		font-size: var(--fs-xs);
	}

	.state {
		display: flex;
		justify-content: center;
		padding: var(--space-10) 0;
		color: var(--ink-faint);
	}

	.empty {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 0.875rem;
		border: 1px solid var(--edge);
		border-radius: var(--radius-md);
		background: var(--surface);
		padding: 4rem 1rem;
		text-align: center;
		color: var(--ink-faint);
	}

	.empty-text {
		display: flex;
		flex-direction: column;
		gap: 0.375rem;
		max-width: 38ch;
	}

	.empty-title {
		font-size: var(--fs-lg);
		font-weight: var(--fw-medium);
		letter-spacing: var(--tracking-tight);
		color: var(--ink);
	}

	.empty-line {
		font-size: var(--fs-sm);
		color: var(--ink-mute);
	}
</style>
