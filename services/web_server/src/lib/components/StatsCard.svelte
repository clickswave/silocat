<script>
	import Icon from '@iconify/svelte';

	let { totalBytes = 10737418240, usedBytes = 0 } = $props();

	function formatSize(bytes) {
		if (bytes === 0) return '0 B';
		const k = 1024;
		const sizes = ['B', 'KB', 'MB', 'GB'];
		const i = Math.min(Math.floor(Math.log(bytes) / Math.log(k)), sizes.length - 1);
		return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
	}

	let freeBytes = $derived(totalBytes - usedBytes);
	let percentage = $derived(totalBytes > 0 ? Math.round((usedBytes / totalBytes) * 100) : 0);
</script>

<div class="stats-card">
	<h3>Storage Usage</h3>
	<button class="more-btn"><Icon icon="ri:more-line" /></button>

	<div class="chart-container">
		<div class="circle outer"></div>
		<div class="circle middle"></div>
		<div class="circle inner">
			<span class="value">{percentage}%</span>
			<span class="label">Used</span>
		</div>
	</div>

	<div class="storage-info">
		<div class="metric">
			<span class="label">Total</span>
			<span class="val">{formatSize(totalBytes)}</span>
		</div>
		<div class="metric">
			<span class="label">Used</span>
			<span class="val">{formatSize(usedBytes)}</span>
		</div>
		<div class="metric">
			<span class="label">Free</span>
			<span class="val">{formatSize(freeBytes)}</span>
		</div>
	</div>

	<button class="upgrade-btn"> Upgrade Plan </button>
</div>

<style lang="scss">
	.stats-card {
		background-color: var(--bg-card);
		border-radius: var(--radius-lg);
		padding: 24px;
		position: relative;
		min-height: 380px;
		display: flex;
		flex-direction: column;
		justify-content: space-between;

		h3 {
			margin: 0;
			font-size: 18px;
			font-weight: 600;
		}

		.more-btn {
			position: absolute;
			top: 24px;
			right: 24px;
			background: transparent;
			border: none;
			color: var(--text-secondary);
			cursor: pointer;
		}

		.chart-container {
			position: relative;
			width: 200px;
			height: 200px;
			margin: 20px auto;
			display: flex;
			align-items: center;
			justify-content: center;

			.circle {
				position: absolute;
				border-radius: 50%;
			}

			.outer {
				width: 100%;
				height: 100%;
				background: conic-gradient(from 180deg, #ff4655, #7b4ae2, #4aa3e2, #ff4655);
				opacity: 0.8;
				filter: blur(20px);
				animation: spin 10s linear infinite;
			}

			.middle {
				width: 85%;
				height: 85%;
				background: var(--bg-card);
				z-index: 2;
			}

			.inner {
				width: 70%;
				height: 70%;
				background: var(--bg-app);
				z-index: 3;
				display: flex;
				flex-direction: column;
				align-items: center;
				justify-content: center;
				box-shadow: inset 0 0 20px rgba(0, 0, 0, 0.5);

				.label {
					font-size: 12px;
					color: var(--text-muted);
					text-transform: uppercase;
					letter-spacing: 1px;
				}

				.value {
					font-size: 24px;
					font-weight: 700;
					color: var(--text-primary);
				}
			}
		}

		.storage-info {
			display: flex;
			justify-content: space-between;
			margin: 20px 0;
			padding: 0 10px;

			.metric {
				display: flex;
				flex-direction: column;
				align-items: center;
				gap: 4px;

				.label {
					font-size: 12px;
					color: var(--text-muted);
				}
				.val {
					font-size: 14px;
					font-weight: 600;
					color: var(--text-primary);
				}
			}
		}

		.upgrade-btn {
			width: 100%;
			padding: 12px;
			background: var(--primary);
			color: #fff;
			border: none;
			border-radius: var(--radius-md);
			font-weight: 600;
			cursor: pointer;
			transition: opacity 0.2s;

			&:hover {
				opacity: 0.9;
			}
		}
	}

	@keyframes spin {
		from {
			transform: rotate(0deg);
		}
		to {
			transform: rotate(360deg);
		}
	}
</style>
