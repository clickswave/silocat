<script>
	import Icon from '@iconify/svelte';
	import { fade } from 'svelte/transition';

	let { data } = $props();
	let requests = $derived(data.requests);

	function formatDate(dateString) {
		return new Date(dateString).toLocaleDateString('en-US', {
			year: 'numeric',
			month: 'short',
			day: 'numeric',
			hour: '2-digit',
			minute: '2-digit'
		});
	}
</script>

<div class="page-header">
	<div class="header-content">
		<h1>Early Access Requests</h1>
		<p class="subtitle">View users who have requested early access</p>
	</div>
</div>

<div class="content-grid">
	<div class="card">
		<div class="table-container">
			<table>
				<thead>
					<tr>
						<th>Email</th>
						<th>Status</th>
						<th>Requested On</th>
					</tr>
				</thead>
				<tbody>
					{#if requests.length === 0}
						<tr>
							<td colspan="3" class="empty-state">
								<Icon icon="ri:inbox-line" width="48" />
								<p>No requests found</p>
							</td>
						</tr>
					{:else}
						{#each requests as req}
							<tr transition:fade>
								<td class="primary-text">{req.email}</td>
								<td>
									<span class="badge" class:pending={req.status === 'pending'}>
										{req.status}
									</span>
								</td>
								<td class="muted-text">{formatDate(req.created_on)}</td>
							</tr>
						{/each}
					{/if}
				</tbody>
			</table>
		</div>
	</div>
</div>

<style lang="scss">
	.page-header {
		margin-bottom: 2rem;

		h1 {
			font-size: 2rem;
			font-weight: 700;
			margin: 0 0 0.5rem 0;
			background: linear-gradient(to right, #fff, #a1a1aa);
			background-clip: text;
			-webkit-background-clip: text;
			-webkit-text-fill-color: transparent;
		}

		.subtitle {
			color: var(--text-secondary);
			margin: 0;
		}
	}

	.card {
		background: var(--bg-card);
		border-radius: var(--radius-lg);
		border: 1px solid var(--border-subtle);
		overflow: hidden;
	}

	.table-container {
		width: 100%;
		overflow-x: auto;

		table {
			width: 100%;
			border-collapse: collapse;
			text-align: left;

			th {
				padding: 1rem 1.5rem;
				color: var(--text-secondary);
				font-weight: 500;
				font-size: 0.875rem;
				border-bottom: 1px solid var(--border-subtle);
				background: rgba(255, 255, 255, 0.02);
			}

			td {
				padding: 1rem 1.5rem;
				border-bottom: 1px solid var(--border-subtle);
				color: var(--text-secondary);
				font-size: 0.95rem;

				&.primary-text {
					color: var(--text-primary);
					font-weight: 500;
				}

				&.muted-text {
					color: var(--text-muted);
					font-size: 0.875rem;
				}

				&.empty-state {
					text-align: center;
					padding: 4rem 2rem;
					color: var(--text-muted);

					p {
						margin-top: 1rem;
					}
				}
			}

			tr:last-child td {
				border-bottom: none;
			}
		}
	}

	.badge {
		padding: 0.25rem 0.75rem;
		border-radius: 9999px;
		font-size: 0.75rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		background: var(--bg-input);
		color: var(--text-secondary);
		border: 1px solid var(--border-subtle);

		&.pending {
			background: rgba(245, 158, 11, 0.1);
			color: var(--warning);
			border-color: rgba(245, 158, 11, 0.2);
		}
	}
</style>
