<script>
	import { onMount } from 'svelte';
	import Icon from '@iconify/svelte';
	import Chart from 'chart.js/auto';

	let { data } = $props();
	let stats = $derived(data.stats || {});
	let recentFiles = $derived(data.recentFiles || []);
	let cloudflare = $derived(data.cloudflare || { shadow: {}, sanctum: {} });
	let chartCanvas;
	let chartInstance;

	function formatBytes(bytes) {
		if (!bytes && bytes !== 0) return '0 B';
		if (bytes === 0) return '0 B';
		const k = 1024;
		const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
		const i = Math.floor(Math.log(bytes) / Math.log(k));
		return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
	}

	onMount(() => {
		if (chartCanvas) {
			chartInstance = new Chart(chartCanvas, {
				type: 'line',
				data: {
					labels: ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'],
					datasets: [
						{
							label: 'Traffic (GB)',
							data: [12, 19, 3, 5, 2, 3, 20], // Mock data for now
							borderColor: '#ff1a1a',
							backgroundColor: 'rgba(255, 26, 26, 0.1)',
							tension: 0.4,
							fill: true
						}
					]
				},
				options: {
					responsive: true,
					maintainAspectRatio: false,
					plugins: {
						legend: { display: false }
					},
					scales: {
						y: {
							beginAtZero: true,
							grid: { color: 'rgba(255, 255, 255, 0.05)' },
							ticks: { color: '#888' }
						},
						x: {
							grid: { display: false },
							ticks: { color: '#888' }
						}
					}
				}
			});
		}
		return () => {
			if (chartInstance) chartInstance.destroy();
		};
	});
</script>

<div class="dashboard-grid">
	<div class="stat-card">
		<div class="icon-box red"><Icon icon="ri:user-line" /></div>
		<div class="info">
			<h3>Total Users</h3>
			<div class="value">{stats.total_users || 0}</div>
		</div>
	</div>

	<div class="stat-card">
		<div class="icon-box blue"><Icon icon="ri:file-line" /></div>
		<div class="info">
			<h3>Total Files</h3>
			<div class="value">{stats.total_files || 0}</div>
		</div>
	</div>

	<div class="stat-card">
		<div class="icon-box green"><Icon icon="ri:hard-drive-line" /></div>
		<div class="info">
			<h3>Storage Used</h3>
			<div class="value">{formatBytes(stats.total_storage_bytes)}</div>
		</div>
	</div>

	<div class="stat-card">
		<div class="icon-box purple"><Icon icon="ri:vip-crown-line" /></div>
		<div class="info">
			<h3>Active Subs</h3>
			<div class="value">{stats.total_subscriptions || 0}</div>
		</div>
	</div>

	<div class="chart-section card">
		<div class="card-header">
			<h2>Network Traffic</h2>
		</div>
		<div class="chart-container">
			<canvas bind:this={chartCanvas}></canvas>
		</div>
	</div>

	<div class="recent-section card">
		<div class="card-header">
			<h2>Recent Files</h2>
			<a href="/dashboard/files" class="view-all">View All</a>
		</div>
		<table>
			<tbody>
				{#each recentFiles as file}
					<tr>
						<td>
							<div class="file-info">
								<span class="name">{file.name}</span>
								<span class="meta">{formatBytes(file.size)}</span>
							</div>
						</td>
						<td class="action">
							<Icon icon="ri:download-line" />
						</td>
					</tr>
				{/each}
			</tbody>
		</table>
	</div>

	<div class="cf-section card">
		<div class="card-header">
			<h2>Cloudflare R2 Usage</h2>
			<div class="icon-orange"><Icon icon="ri:cloud-line" /></div>
		</div>
		<div class="cf-grid">
			<div class="cf-card">
				<h4>Shadow Bucket</h4>
				<div class="details">
					<div class="row">
						<span>Objects</span>
						<strong
							>{cloudflare.shadow?.object_count >= 0
								? cloudflare.shadow.object_count
								: 'Error'}</strong
						>
					</div>
					<div class="row">
						<span>Size</span>
						<strong
							>{cloudflare.shadow?.total_size_bytes >= 0
								? formatBytes(cloudflare.shadow.total_size_bytes)
								: 'Error'}</strong
						>
					</div>
				</div>
			</div>
			<div class="cf-card">
				<h4>Sanctum Bucket</h4>
				<div class="details">
					<div class="row">
						<span>Objects</span>
						<strong
							>{cloudflare.sanctum?.object_count >= 0
								? cloudflare.sanctum.object_count
								: 'Error'}</strong
						>
					</div>
					<div class="row">
						<span>Size</span>
						<strong
							>{cloudflare.sanctum?.total_size_bytes >= 0
								? formatBytes(cloudflare.sanctum.total_size_bytes)
								: 'Error'}</strong
						>
					</div>
				</div>
			</div>
		</div>
	</div>
</div>

<style lang="scss">
	.dashboard-grid {
		display: grid;
		grid-template-columns: repeat(4, 1fr);
		grid-auto-rows: auto;
		gap: 1.5rem;
	}

	.cf-section {
		grid-column: span 4;
		.card-header {
			display: flex;
			gap: 1rem;
			justify-content: flex-start;
			.icon-orange {
				color: #f97316;
				font-size: 1.5rem;
				display: flex;
				align-items: center;
			}
		}
		.cf-grid {
			display: grid;
			grid-template-columns: 1fr 1fr;
			gap: 1.5rem;
			.cf-card {
				background: var(--bg-input);
				border-radius: var(--radius-md);
				padding: 1rem;
				h4 {
					margin: 0 0 1rem 0;
					color: var(--text-secondary);
					font-size: 1rem;
					border-bottom: 1px solid var(--border-subtle);
					padding-bottom: 0.5rem;
				}
				.details {
					display: flex;
					flex-direction: column;
					gap: 0.5rem;
					.row {
						display: flex;
						justify-content: space-between;
						font-size: 0.9rem;
						span {
							color: var(--text-muted);
						}
						strong {
							color: var(--text-primary);
						}
					}
				}
			}
		}
	}

	.stat-card {
		background: var(--bg-card);
		border: 1px solid var(--border-default);
		border-radius: var(--radius-lg);
		padding: 1.5rem;
		display: flex;
		align-items: center;
		gap: 1rem;
		box-shadow: var(--shadow-card);

		.icon-box {
			width: 48px;
			height: 48px;
			border-radius: 12px;
			display: flex;
			align-items: center;
			justify-content: center;
			font-size: 1.5rem;
			&.red {
				background: rgba(255, 26, 26, 0.1);
				color: var(--primary);
			}
			&.blue {
				background: rgba(59, 130, 246, 0.1);
				color: #3b82f6;
			}
			&.green {
				background: rgba(16, 185, 129, 0.1);
				color: #10b981;
			}
			&.purple {
				background: rgba(139, 92, 246, 0.1);
				color: #8b5cf6;
			}
		}

		.info {
			h3 {
				margin: 0;
				font-size: 0.85rem;
				color: var(--text-muted);
				font-weight: 500;
			}
			.value {
				font-size: 1.5rem;
				font-weight: 700;
				color: var(--text-primary);
				margin-top: 0.2rem;
			}
		}
	}

	.card {
		background: var(--bg-card);
		border: 1px solid var(--border-default);
		border-radius: var(--radius-lg);
		padding: 1.5rem;
		box-shadow: var(--shadow-card);
	}

	.card-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 1.5rem;
		h2 {
			font-size: 1.2rem;
			font-weight: 700;
			margin: 0;
		}
		.view-all {
			font-size: 0.85rem;
			color: var(--primary);
			text-decoration: none;
			font-weight: 600;
		}
	}

	.chart-section {
		grid-column: span 3;
		height: 400px;
		.chart-container {
			height: 320px;
			position: relative;
		}
	}

	.recent-section {
		grid-column: span 1;
		table {
			width: 100%;
			border-collapse: collapse;
			tr {
				border-bottom: 1px solid var(--border-subtle);
				&:last-child {
					border-bottom: none;
				}
			}
			td {
				padding: 1rem 0;
			}
			.file-info {
				display: flex;
				flex-direction: column;
				gap: 0.2rem;
				.name {
					font-weight: 600;
					color: var(--text-primary);
					font-size: 0.9rem;
					white-space: nowrap;
					overflow: hidden;
					text-overflow: ellipsis;
					max-width: 200px;
					display: inline-block;
				}
				.meta {
					color: var(--text-muted);
					font-size: 0.75rem;
				}
			}
			.action {
				text-align: right;
				color: var(--text-muted);
			}
		}
	}
</style>
