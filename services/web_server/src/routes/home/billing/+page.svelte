<script>
	import Icon from '@iconify/svelte';
	import { FrontendClient } from '$lib/frontendClient.js';
	import { onMount } from 'svelte';
	import { toast } from 'svelte-sonner';

	let { data } = $props();

	let plan = $state({
		name: 'Free Plan',
		price: '$0.00',
		period: 'month',
		status: 'Active',
		nextBilling: 'N/A'
	});

	let usage = $state({
		storage: 0,
		limit: 50,
		bandwidth: 'Unlimited',
		bandwidthLimit: 'Unlimited'
	});

	let invoices = $state([]);
	let loading = $state(true);

	// Initialize usage from user data or defaults
	$effect(() => {
		if (data.user) {
			// Plan
			if (data.user.subscription?.name === 'Pro') {
				plan.name = 'Pro Plan';
				plan.price = '$9.00'; // Or adjust currency
				plan.status = data.user.subscription.status || 'Active';

				if (data.user.subscription.expires_on) {
					plan.nextBilling = new Date(data.user.subscription.expires_on).toLocaleDateString();
				}
			} else {
				plan.name = 'Free Plan';
				plan.price = '$0.00';
			}

			// Usage
			// Convert bytes to GB
			let limitBytes = data.user.default_storage_bytes || 53687091200; // 50GB default
			if (data.user.subscription?.additional_space) {
				limitBytes += data.user.subscription.additional_space;
			}

			// We might need an endpoint for current usage, but for now we can default to 0 or
			// if passed in data load. Assuming 0 or small mock for now if not available.
			// Ideally we fetch from `/api/v1/shadow/file` stats or similar.
			// Using 0 as placeholder or if backend sends it.
			usage.limit = (limitBytes / (1024 * 1024 * 1024)).toFixed(0);
			usage.storage = data.user.storage_used_gb || 0; // Assuming this might be added to user object?
			// If not, we just show 0 or handle it.
		}
	});

	onMount(async () => {
		try {
			const res = await FrontendClient.get('/api/v1/billing/history');
			if (res.data.status === 200) {
				invoices = res.data.data.orders.map((order) => ({
					id: order.reference_id || order.id.substring(0, 8),
					date: new Date(order.created_on).toLocaleDateString(),
					amount: `${order.currency === 'USD' ? '$' : '₹'}${order.amount / 100}`,
					status: order.status,
					currency: order.currency,
					raw_amount: order.amount,
					gateway: order.payment_gateway
				}));
			}
		} catch (e) {
			console.error('Failed to fetch history', e);
		} finally {
			loading = false;
		}
	});

	function getStoragePercentage() {
		return (usage.storage / usage.limit) * 100;
	}

	function downloadInvoice(invoice) {
		// Simple mock download/print
		const content = `
			INVOICE #${invoice.id}
			----------------------
			Date: ${invoice.date}
			Status: ${invoice.status}
			
			Item: SiloCat Subscription
			Amount: ${invoice.amount}
			
			Thank you for your business.
			ClisksWave Labs Private Limited
		`;

		const blob = new Blob([content], { type: 'text/plain' });
		const url = window.URL.createObjectURL(blob);
		const a = document.createElement('a');
		a.href = url;
		a.download = `invoice_${invoice.id}.txt`;
		a.click();
		window.URL.revokeObjectURL(url);
		toast.success('Invoice downloaded');
	}
</script>

<div class="billing-page">
	<header class="page-header">
		<h1>Billing & Subscription</h1>
		<p class="subtitle">Manage your plan and view order history</p>
	</header>

	<div class="billing-grid">
		<!-- Current Plan -->
		<div class="card plan-card">
			<div class="card-header">
				<div class="header-icon">
					<Icon icon="ri:vip-crown-2-line" width="24" />
				</div>
				<h2>Current Plan</h2>
				<span class="status-badge {plan.status === 'Active' ? 'active' : ''}">{plan.status}</span>
			</div>

			<div class="plan-details">
				<div class="plan-name">{plan.name}</div>
				<div class="plan-price">
					<span class="currency">{plan.price}</span>
					<span class="period">/{plan.period}</span>
				</div>
				<p class="next-billing">Next billing date: <span>{plan.nextBilling}</span></p>
			</div>

			<div class="card-actions">
				<button
					class="btn btn-outline"
					onclick={() => (window.location.href = '/home/subscription')}>Change Plan</button
				>
				<!-- Cancel button hidden for now until implemented -->
			</div>
		</div>

		<!-- Usage Stats -->
		<div class="card usage-card">
			<div class="card-header">
				<div class="header-icon">
					<Icon icon="ri:hard-drive-2-line" width="24" />
				</div>
				<h2>Storage Usage</h2>
			</div>

			<div class="usage-stats">
				<div class="stat-item">
					<div class="stat-header">
						<span>Storage</span>
						<span class="value">{usage.storage} GB / {usage.limit} GB</span>
					</div>
					<div class="progress-bar">
						<div class="fill" style="width: {getStoragePercentage()}%"></div>
					</div>
				</div>

				<div class="feature-list">
					<div class="feature">
						<Icon icon="ri:checkbox-circle-fill" class="check-icon" />
						<span>Unlimited Bandwidth</span>
					</div>
					<div class="feature">
						<Icon icon="ri:checkbox-circle-fill" class="check-icon" />
						<span>Priority Support</span>
					</div>
					<div class="feature">
						<Icon icon="ri:checkbox-circle-fill" class="check-icon" />
						<span>Password Protection</span>
					</div>
				</div>
			</div>
		</div>

		<!-- Order History -->
		<div class="card invoices-card">
			<div class="card-header">
				<div class="header-icon">
					<Icon icon="ri:file-list-3-line" width="24" />
				</div>
				<h2>Order History</h2>
			</div>

			<div class="table-container">
				{#if loading}
					<div class="loading-state">Loading history...</div>
				{:else if invoices.length === 0}
					<div class="empty-state">No orders found.</div>
				{:else}
					<table class="invoices-table">
						<thead>
							<tr>
								<th>Order ID</th>
								<th>Date</th>
								<th>Amount</th>
								<th>Status</th>
								<th></th>
							</tr>
						</thead>
						<tbody>
							{#each invoices as invoice}
								<tr>
									<td class="id">{invoice.id}</td>
									<td>{invoice.date}</td>
									<td>{invoice.amount}</td>
									<td
										><span class="status-pill {invoice.status.toLowerCase()}">{invoice.status}</span
										></td
									>
									<td class="action">
										<button
											class="icon-btn"
											title="Download Invoice"
											onclick={() => downloadInvoice(invoice)}
										>
											<Icon icon="ri:download-line" />
										</button>
									</td>
								</tr>
							{/each}
						</tbody>
					</table>
				{/if}
			</div>
		</div>
	</div>
</div>

<style lang="scss">
	.billing-page {
		padding: 2rem;
		max-width: 1200px;
		margin: 0 auto;
		animation: fade-in 0.3s ease-out;
	}

	@keyframes fade-in {
		from {
			opacity: 0;
			transform: translateY(10px);
		}
		to {
			opacity: 1;
			transform: translateY(0);
		}
	}

	.page-header {
		margin-bottom: 2rem;

		h1 {
			font-size: 2rem;
			font-weight: 700;
			margin-bottom: 0.5rem;
			color: var(--text-primary);
		}

		.subtitle {
			color: var(--text-muted);
			font-size: 1rem;
		}
	}

	.billing-grid {
		display: grid;
		grid-template-columns: repeat(2, 1fr);
		gap: 1.5rem;

		@media (max-width: 900px) {
			grid-template-columns: 1fr;
		}
	}

	.card {
		background: var(--bg-card);
		border: 1px solid var(--border-default);
		border-radius: 16px;
		padding: 1.5rem;
		display: flex;
		flex-direction: column;

		.card-header {
			display: flex;
			align-items: center;
			gap: 1rem;
			margin-bottom: 1.5rem;

			.header-icon {
				width: 40px;
				height: 40px;
				border-radius: 10px;
				background: rgba(255, 70, 85, 0.1);
				color: var(--primary, #ff4655);
				display: flex;
				align-items: center;
				justify-content: center;
			}

			h2 {
				font-size: 1.25rem;
				font-weight: 600;
				margin: 0;
				flex: 1;
				color: var(--text-primary);
			}
		}
	}

	/* Plan Card */
	.plan-card {
		.status-badge {
			background: rgba(61, 220, 151, 0.15);
			color: #059669;
			padding: 0.25rem 0.75rem;
			border-radius: 20px;
			font-size: 0.8rem;
			font-weight: 600;
			text-transform: uppercase;
		}

		.plan-details {
			margin-bottom: 2rem;

			.plan-name {
				font-size: 1.1rem;
				color: var(--text-muted);
				margin-bottom: 0.5rem;
			}

			.plan-price {
				display: flex;
				align-items: baseline;
				gap: 0.25rem;
				margin-bottom: 1rem;

				.currency {
					font-size: 2.5rem;
					font-weight: 700;
					color: var(--text-primary);
				}

				.period {
					color: var(--text-muted);
					font-size: 1rem;
				}
			}

			.next-billing {
				color: var(--text-muted);
				font-size: 0.9rem;

				span {
					color: var(--text-primary);
					font-weight: 500;
				}
			}
		}

		.card-actions {
			margin-top: auto;
			display: flex;
			gap: 1rem;
		}
	}

	/* Usage Card */
	.usage-card {
		.usage-stats {
			display: flex;
			flex-direction: column;
			gap: 1.5rem;
			flex: 1;
		}
		/* ... skipped usage card stuff as it was handled ... wait, I need to match the StartLine properly so I don't overwrite if I don't include it. */
		/* Ah, I can just target the plan-card text logic specifically if I narrow the range */
		/* But I see Button styles at the bottom too. Let's do two chunks if needed or just one huge one? */
		/* The previous tool call ended at 472. */
		/* I'll just do the plan-card details and buttons. */
		/* Wait, Plan Card was around 328. Buttons around 529. */
		/* I'll use multi_replace for efficiency or just target them. */
		/* Let's double check line numbers. */
		/* Plan Card details: 328 to 365 */
		/* Buttons: 543 to 551 */
		/* I'll use replace_file_content for Plan Card first since it's cleaner. */

		.card-actions {
			margin-top: auto;
			display: flex;
			gap: 1rem;
		}
	}

	/* Usage Card */
	.usage-card {
		.usage-stats {
			display: flex;
			flex-direction: column;
			gap: 1.5rem;
			flex: 1;
		}

		.stat-item {
			.stat-header {
				display: flex;
				justify-content: space-between;
				font-size: 0.9rem;
				color: var(--text-muted);
				margin-bottom: 0.5rem;

				.value {
					color: var(--text-primary);
					font-weight: 500;
				}
			}

			.progress-bar {
				height: 8px;
				background: var(--bg-input);
				border-radius: 4px;
				overflow: hidden;

				.fill {
					height: 100%;
					background: var(--primary, #ff4655);
					border-radius: 4px;
				}
			}
		}

		.feature-list {
			margin-top: auto;
			display: flex;
			flex-direction: column;
			gap: 0.75rem;

			.feature {
				display: flex;
				align-items: center;
				gap: 0.75rem;
				color: var(--text-muted);
				font-size: 0.9rem;

				.check-icon {
					color: var(--primary, #ff4655);
				}
			}
		}
	}

	/* Invoices Card */
	.invoices-card {
		grid-column: span 2;

		@media (max-width: 900px) {
			grid-column: span 1;
		}

		.table-container {
			overflow-x: auto;
		}

		.loading-state,
		.empty-state {
			color: var(--text-muted);
			text-align: center;
			padding: 2rem;
			font-style: italic;
		}

		.invoices-table {
			width: 100%;
			border-collapse: collapse;
			font-size: 0.95rem;

			th {
				text-align: left;
				padding: 1rem;
				color: var(--text-muted);
				font-weight: 500;
				font-size: 0.85rem;
				border-bottom: 1px solid var(--border-default);
			}

			td {
				padding: 1rem;
				color: var(--text-secondary);
				border-bottom: 1px solid var(--border-default);

				&.id {
					font-family: monospace;
					color: var(--text-primary);
				}

				&.action {
					text-align: right;
				}
			}

			tr:last-child td {
				border-bottom: none;
			}

			.status-pill {
				padding: 0.25rem 0.75rem;
				border-radius: 20px;
				font-size: 0.8rem;
				font-weight: 500;
				text-transform: capitalization;
				background: rgba(255, 255, 255, 0.05);
				color: var(--text-muted);

				&.completed {
					background: rgba(61, 220, 151, 0.1);
					color: #059669;
				}

				&.paid {
					background: rgba(61, 220, 151, 0.1);
					color: #059669;
				}

				&.pending {
					background: rgba(253, 224, 71, 0.1);
					color: #d97706;
				}

				&.failed {
					background: rgba(239, 68, 68, 0.1);
					color: #dc2626;
				}
			}

			.icon-btn {
				background: none;
				border: none;
				color: var(--text-muted);
				cursor: pointer;
				padding: 0.25rem;
				transition: color 0.2s;

				&:hover {
					color: var(--text-primary);
				}
			}
		}
	}

	/* Buttons */
	.btn {
		padding: 0.75rem 1.5rem;
		border-radius: 8px;
		font-weight: 500;
		cursor: pointer;
		transition: all 0.2s;
		font-size: 0.9rem;

		&.small {
			padding: 0.5rem 1rem;
			font-size: 0.85rem;
		}
	}

	.btn-outline {
		background: transparent;
		border: 1px solid var(--border-default);
		color: var(--text-primary);

		&:hover {
			border-color: var(--text-primary);
			background: var(--nav-hover);
		}
	}

	.btn-text {
		background: transparent;
		border: none;
		color: #ef4444;

		&:hover {
			text-decoration: underline;
		}
	}
</style>
