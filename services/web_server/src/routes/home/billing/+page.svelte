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
		limit: 10,
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
				plan.price = '$10.00';
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
			let limitBytes = data.user.default_storage_bytes || 10737418240; // 10GB default
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
					amount: `${{ USD: '$', EUR: '€', INR: '₹' }[order.currency] || ''}${order.amount / 100}`,
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
			
			Item: Silocat Subscription
			Amount: ${invoice.amount}
			
			Thank you for your business.
			Clickswave Labs Private Limited
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
		width: 100%;
	}

	.page-header {
		margin-bottom: var(--space-6);

		h1 {
			font-size: var(--fs-h3);
			font-weight: var(--fw-semibold);
			margin-bottom: var(--space-1);
			color: var(--text-primary);
		}

		.subtitle {
			color: var(--text-muted);
			font-size: var(--fs-sm);
		}
	}

	.billing-grid {
		display: grid;
		grid-template-columns: repeat(2, 1fr);
		gap: var(--space-5);

		@media (max-width: 900px) {
			grid-template-columns: 1fr;
		}
	}

	.card {
		background: var(--bg-card);
		border: 1px solid var(--border-default);
		border-radius: var(--radius-md);
		box-shadow: var(--shadow-card);
		padding: var(--space-5);
		display: flex;
		flex-direction: column;

		.card-header {
			display: flex;
			align-items: center;
			gap: var(--space-4);
			margin-bottom: var(--space-5);

			.header-icon {
				width: 40px;
				height: 40px;
				border-radius: var(--radius-sm);
				background: var(--tint-soft);
				color: var(--primary);
				display: flex;
				align-items: center;
				justify-content: center;
			}

			h2 {
				font-size: var(--fs-h3);
				font-weight: var(--fw-semibold);
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
			color: var(--success);
			padding: var(--space-1) var(--space-3);
			border-radius: var(--radius-pill);
			font-size: var(--fs-xs);
			font-weight: var(--fw-semibold);
			text-transform: uppercase;
		}

		.plan-details {
			margin-bottom: var(--space-6);

			.plan-name {
				font-size: var(--fs-lg);
				color: var(--text-secondary);
				margin-bottom: var(--space-2);
			}

			.plan-price {
				display: flex;
				align-items: baseline;
				gap: var(--space-1);
				margin-bottom: var(--space-4);

				.currency {
					font-size: var(--fs-h1);
					font-weight: var(--fw-bold);
					font-family: var(--font-mono);
					color: var(--text-primary);
				}

				.period {
					color: var(--text-muted);
					font-size: var(--fs-body);
				}
			}

			.next-billing {
				color: var(--text-muted);
				font-size: var(--fs-sm);

				span {
					color: var(--text-primary);
					font-weight: var(--fw-medium);
				}
			}
		}

		.card-actions {
			margin-top: auto;
			display: flex;
			gap: var(--space-4);
		}
	}

	/* Usage Card */
	.usage-card {
		.usage-stats {
			display: flex;
			flex-direction: column;
			gap: var(--space-5);
			flex: 1;
		}

		.stat-item {
			.stat-header {
				display: flex;
				justify-content: space-between;
				font-size: var(--fs-sm);
				color: var(--text-secondary);
				margin-bottom: var(--space-2);

				.value {
					color: var(--text-primary);
					font-weight: var(--fw-medium);
					font-family: var(--font-mono);
				}
			}

			.progress-bar {
				height: 8px;
				background: var(--bg-input);
				border-radius: var(--radius-pill);
				overflow: hidden;

				.fill {
					height: 100%;
					background: var(--accent-gradient);
					border-radius: var(--radius-pill);
					transition: width var(--dur) var(--ease);
				}
			}
		}

		.feature-list {
			margin-top: auto;
			display: flex;
			flex-direction: column;
			gap: var(--space-3);

			.feature {
				display: flex;
				align-items: center;
				gap: var(--space-3);
				color: var(--text-secondary);
				font-size: var(--fs-sm);

				.check-icon {
					color: var(--primary);
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
			padding: var(--space-6);
		}

		.invoices-table {
			width: 100%;
			border-collapse: collapse;
			font-size: var(--fs-sm);

			th {
				text-align: left;
				padding: var(--space-4);
				color: var(--text-secondary);
				font-weight: var(--fw-medium);
				font-size: var(--fs-xs);
				text-transform: uppercase;
				letter-spacing: 0.06em;
				border-bottom: 1px solid var(--border-default);
			}

			td {
				padding: var(--space-4);
				color: var(--text-secondary);
				border-bottom: 1px solid var(--hairline);

				&.id {
					font-family: var(--font-mono);
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
				padding: var(--space-1) var(--space-3);
				border-radius: var(--radius-pill);
				font-size: var(--fs-xs);
				font-weight: var(--fw-medium);
				text-transform: capitalize;
				background: var(--tint-soft);
				color: var(--text-secondary);

				&.completed {
					background: rgba(61, 220, 151, 0.12);
					color: var(--success);
				}

				&.paid {
					background: rgba(61, 220, 151, 0.12);
					color: var(--success);
				}

				&.pending {
					background: rgba(242, 201, 76, 0.12);
					color: var(--warning);
				}

				&.failed {
					background: var(--danger-soft);
					color: var(--danger);
				}
			}

			.icon-btn {
				background: none;
				border: none;
				color: var(--text-muted);
				cursor: pointer;
				padding: var(--space-1);
				transition: color var(--dur) var(--ease);

				&:hover {
					color: var(--text-primary);
				}
			}
		}
	}

	/* Buttons */
	.btn {
		padding: 0.7rem 1.25rem;
		border-radius: var(--radius-pill);
		font-weight: var(--fw-semibold);
		cursor: pointer;
		transition: background var(--dur) var(--ease), border-color var(--dur) var(--ease);
		font-size: var(--fs-sm);

		&.small {
			padding: var(--space-2) var(--space-4);
			font-size: var(--fs-sm);
		}
	}

	.btn-outline {
		background: var(--tint-soft);
		border: 1px solid var(--border-default);
		color: var(--text-primary);

		&:hover {
			border-color: var(--border-strong);
			background: var(--tint-softer);
		}
	}

	.btn-text {
		background: transparent;
		border: none;
		color: var(--danger);

		&:hover {
			text-decoration: underline;
		}
	}
</style>
