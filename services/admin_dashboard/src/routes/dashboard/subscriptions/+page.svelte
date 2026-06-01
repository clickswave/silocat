<script>
	import Icon from '@iconify/svelte';
	import { createQuery } from '@tanstack/svelte-query';

	let { data } = $props();

	let searchQuery = $state('');

	const subsQuery = createQuery(() => ({
		queryKey: ['subscriptions'],
		queryFn: async () => {
			const res = await fetch('/api/subscriptions');
			if (!res.ok) throw new Error('Failed to fetch');
			const json = await res.json();
			return json.subscriptions || [];
		},
		initialData: data.subscriptions,
		refetchInterval: 5000
	}));

	let subscriptions = $derived(subsQuery.data || []);

	let filteredSubs = $derived.by(() => {
		if (!searchQuery) return subscriptions;
		const q = searchQuery.toLowerCase();
		return subscriptions.filter((s) => s.name.toLowerCase().includes(q));
	});

	function formatDate(dateStr) {
		if (!dateStr) return '-';
		return new Date(dateStr).toLocaleDateString();
	}

	function formatBytes(bytes) {
		if (bytes === 0) return '0 B';
		const k = 1024;
		const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
		const i = Math.floor(Math.log(bytes) / Math.log(k));
		return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
	}
</script>

<div class="page-header">
	<div class="header-content">
		<div>
			<h1>Subscriptions</h1>
			<p>View active subscriptions.</p>
		</div>
	</div>
</div>

<div class="card list">
	<div class="card-header">
		<div class="header-left">
			<h2>All Subscriptions</h2>
			<div class="badge">{filteredSubs.length} Total</div>
			{#if subsQuery.isFetching}
				<Icon icon="ri:loader-4-line" class="spin" />
			{/if}
		</div>
		<div class="header-right">
			<div class="search-box">
				<Icon icon="ri:search-line" class="search-icon" />
				<input type="text" placeholder="Search by name..." bind:value={searchQuery} />
			</div>
		</div>
	</div>

	<div class="table-container">
		<table>
			<thead>
				<tr>
					<th>Name</th>
					<th>Additional Space</th>
					<th>Created By</th>
					<th>Created On</th>
					<th>Expires On</th>
				</tr>
			</thead>
			<tbody>
				{#each filteredSubs as sub}
					<tr>
						<td><span class="user-pill">{sub.name}</span></td>
						<td class="muted">{formatBytes(sub.additional_space * 1024 * 1024 * 1024)}</td>
						<td class="muted">{sub.created_by}</td>
						<td class="muted">{formatDate(sub.created_on)}</td>
						<td class="muted">{formatDate(sub.expires_on)}</td>
					</tr>
				{/each}
			</tbody>
		</table>
	</div>
</div>

<style lang="scss">
	.spin {
		animation: spin 1s linear infinite;
	}
	@keyframes spin {
		100% {
			transform: rotate(360deg);
		}
	}

	.page-header {
		margin-bottom: 2rem;
		.header-content {
			display: flex;
			justify-content: space-between;
			align-items: center;
		}
		h1 {
			font-size: 2rem;
			font-weight: 700;
			margin: 0 0 0.5rem 0;
		}
		p {
			color: var(--text-muted);
			font-size: 1rem;
			margin: 0;
		}
	}

	.card {
		background: var(--bg-card);
		border: 1px solid var(--border-default);
		border-radius: var(--radius-lg);
		padding: 1.5rem;
		box-shadow: var(--shadow-card);
	}

	.list {
		.card-header {
			display: flex;
			justify-content: space-between;
			align-items: center;
			margin-bottom: 2rem;
			.header-left {
				display: flex;
				align-items: center;
				gap: 1rem;
			}
			h2 {
				margin: 0;
				font-size: 1.2rem;
				font-weight: 700;
				letter-spacing: -0.02em;
			}
			.badge {
				background: var(--bg-card-hover);
				padding: 0.25rem 0.75rem;
				border-radius: 20px;
				font-size: 0.8rem;
				color: var(--text-secondary);
				border: 1px solid var(--border-subtle);
			}
			.search-box {
				position: relative;
				.search-icon {
					position: absolute;
					left: 1rem;
					top: 50%;
					transform: translateY(-50%);
					color: var(--text-muted);
					pointer-events: none;
				}
				input {
					background: var(--bg-input);
					border: 1px solid var(--border-default);
					padding: 0.7rem 1rem 0.7rem 2.8rem;
					border-radius: var(--radius-md);
					color: var(--text-primary);
					font-family: inherit;
					outline: none;
					min-width: 300px;
					transition: all 0.2s;
					&:focus {
						border-color: var(--primary);
						box-shadow: 0 0 0 2px rgba(255, 26, 26, 0.1);
					}
					&::placeholder {
						color: var(--text-muted);
					}
				}
			}
		}
		.table-container {
			overflow-x: auto;
			table {
				width: 100%;
				border-collapse: separate;
				border-spacing: 0;
				text-align: left;
				thead th {
					padding: 1rem;
					color: var(--text-muted);
					font-weight: 600;
					font-size: 0.8rem;
					text-transform: uppercase;
					letter-spacing: 0.05em;
					border-bottom: 1px solid var(--border-default);
				}
				tbody tr {
					transition: background 0.1s;
					&:hover {
						background: var(--bg-card-hover);
					}
				}
				td {
					padding: 1.2rem 1rem;
					border-bottom: 1px solid var(--border-subtle);
					font-size: 0.95rem;
					color: var(--text-secondary);
				}
				tr:last-child td {
					border-bottom: none;
				}
				.user-pill {
					font-weight: 600;
					color: var(--text-primary);
				}
				.muted {
					color: var(--text-muted);
					font-size: 0.85rem;
				}
			}
		}
	}
</style>
