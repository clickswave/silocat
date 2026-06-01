<script>
	import Icon from '@iconify/svelte';
	import { createQuery } from '@tanstack/svelte-query';

	let { data } = $props();

	let searchQuery = $state('');

	const ordersQuery = createQuery(() => ({
		queryKey: ['orders'],
		queryFn: async () => {
			const res = await fetch('/api/orders');
			if (!res.ok) throw new Error('Failed to fetch');
			const json = await res.json();
			return json.orders || [];
		},
		initialData: data.orders,
		refetchInterval: 5000
	}));

	let orders = $derived(ordersQuery.data || []);

	let filteredOrders = $derived.by(() => {
		if (!searchQuery) return orders;
		const q = searchQuery.toLowerCase();
		return orders.filter((o) => o.reference_id.toLowerCase().includes(q));
	});

	function formatDate(dateStr) {
		if (!dateStr) return '-';
		return new Date(dateStr).toLocaleString();
	}

	function formatAmount(amount, currency) {
		// Assuming amount is in cents if USD, etc. But schema says INT.
		// Let's assume smallest unit.
		return (amount / 100).toFixed(2) + ' ' + currency.toUpperCase();
	}
</script>

<div class="page-header">
	<div class="header-content">
		<div>
			<h1>Orders</h1>
			<p>View transaction history.</p>
		</div>
	</div>
</div>

<div class="card list">
	<div class="card-header">
		<div class="header-left">
			<h2>All Orders</h2>
			<div class="badge">{filteredOrders.length} Total</div>
			{#if ordersQuery.isFetching}
				<Icon icon="ri:loader-4-line" class="spin" />
			{/if}
		</div>
		<div class="header-right">
			<div class="search-box">
				<Icon icon="ri:search-line" class="search-icon" />
				<input type="text" placeholder="Search reference ID..." bind:value={searchQuery} />
			</div>
		</div>
	</div>

	<div class="table-container">
		<table>
			<thead>
				<tr>
					<th>Reference ID</th>
					<th>Amount</th>
					<th>Gateway</th>
					<th>Status</th>
					<th>Date</th>
				</tr>
			</thead>
			<tbody>
				{#each filteredOrders as order}
					<tr>
						<td><code class="code-pill">{order.reference_id}</code></td>
						<td class="amount">{formatAmount(order.amount, order.currency)}</td>
						<td class="muted">{order.payment_gateway}</td>
						<td>
							<span
								class="status-pill"
								class:completed={order.status === 'completed'}
								class:failed={order.status === 'failed'}
							>
								{order.status}
							</span>
						</td>
						<td class="muted">{formatDate(order.created_on)}</td>
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
				.amount {
					font-weight: 600;
					color: var(--text-primary);
				}
				.code-pill {
					font-family: 'JetBrains Mono', monospace;
					background: rgba(255, 255, 255, 0.05);
					color: var(--text-primary);
					padding: 0.4rem 0.8rem;
					border-radius: 6px;
					font-size: 0.85rem;
					border: 1px solid transparent;
					transition: all 0.2s;
					&:hover {
						border-color: var(--border-default);
						background: var(--bg-card-hover);
					}
				}
				.muted {
					color: var(--text-muted);
					font-size: 0.85rem;
				}
				.status-pill {
					font-size: 0.75rem;
					padding: 0.25rem 0.75rem;
					border-radius: 20px;
					background: var(--bg-card-hover);
					text-transform: uppercase;
					font-weight: 700;
					letter-spacing: 0.05em;
					border: 1px solid var(--border-subtle);
					color: var(--text-muted);
					&.completed {
						background: rgba(16, 185, 129, 0.1);
						color: var(--success);
						border-color: rgba(16, 185, 129, 0.2);
					}
					&.failed {
						background: rgba(255, 26, 26, 0.1);
						color: var(--danger);
						border-color: rgba(255, 26, 26, 0.2);
					}
				}
			}
		}
	}
</style>
