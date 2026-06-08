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
		border: 1px solid var(--border-default);
		border-radius: var(--radius-md);
		box-shadow: var(--shadow-card);
		padding: var(--space-5);
		position: relative;
		min-height: 380px;
		display: flex;
		flex-direction: column;
		justify-content: space-between;

		h3 {
			margin: 0;
			font-size: var(--fs-h3);
			font-weight: var(--fw-semibold);
		}

		.more-btn {
			position: absolute;
			top: var(--space-5);
			right: var(--space-5);
			background: transparent;
			border: none;
			color: var(--text-secondary);
			cursor: pointer;
			display: flex;
		}

		.chart-container {
			position: relative;
			width: 200px;
			height: 200px;
			margin: var(--space-5) auto;
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
					font-size: var(--fs-xs);
					color: var(--text-muted);
					text-transform: uppercase;
					letter-spacing: 0.08em;
				}

				.value {
					font-size: var(--fs-h3);
					font-weight: var(--fw-bold);
					color: var(--text-primary);
				}
			}
		}

		.storage-info {
			display: flex;
			justify-content: space-between;
			margin: var(--space-5) 0;
			padding: 0 var(--space-2);

			.metric {
				display: flex;
				flex-direction: column;
				align-items: center;
				gap: var(--space-1);

				.label {
					font-size: var(--fs-xs);
					color: var(--text-muted);
				}
				.val {
					font-size: var(--fs-sm);
					font-weight: var(--fw-semibold);
					color: var(--text-primary);
				}
			}
		}

		.upgrade-btn {
			width: 100%;
			padding: 0.7rem 1.25rem;
			background: var(--accent-gradient);
			color: #fff;
			border: 1px solid transparent;
			border-radius: var(--radius-pill);
			font-family: inherit;
			font-size: var(--fs-body);
			font-weight: var(--fw-semibold);
			cursor: pointer;
			box-shadow: 0 6px 20px -6px var(--primary-glow);
			transition: filter var(--dur) var(--ease), box-shadow var(--dur) var(--ease);

			&:hover {
				filter: brightness(1.06);
				box-shadow: 0 10px 28px -6px var(--primary-glow);
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
