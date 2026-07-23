<script>
	import { Progress, Button } from '$lib/ui';

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
	<div class="head">
		<h3>Storage</h3>
		<span class="pct">{percentage}%</span>
	</div>

	<Progress value={percentage} size="md" tone={percentage > 90 ? 'warn' : 'accent'} label="Storage used" />

	<div class="metrics">
		<div class="metric">
			<span class="label">Used</span>
			<span class="val">{formatSize(usedBytes)}</span>
		</div>
		<div class="metric">
			<span class="label">Free</span>
			<span class="val">{formatSize(freeBytes)}</span>
		</div>
		<div class="metric">
			<span class="label">Total</span>
			<span class="val">{formatSize(totalBytes)}</span>
		</div>
	</div>

	<Button variant="ghost" size="sm" href="/home/subscription">Upgrade</Button>
</div>

<style lang="scss">
	.stats-card {
		background: var(--surface);
		border: 1px solid var(--edge);
		border-radius: var(--radius-md);
		padding: var(--space-5);
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
	}

	.head {
		display: flex;
		align-items: baseline;
		justify-content: space-between;

		h3 {
			font-size: var(--fs-body);
			font-weight: var(--fw-semibold);
			margin: 0;
		}

		.pct {
			font-family: var(--font-mono);
			font-size: var(--fs-sm);
			color: var(--ink-mute);
		}
	}

	.metrics {
		display: flex;
		justify-content: space-between;

		.metric {
			display: flex;
			flex-direction: column;
			gap: 2px;

			.label {
				font-size: var(--fs-xs);
				color: var(--ink-faint);
			}
			.val {
				font-family: var(--font-mono);
				font-size: var(--fs-sm);
				color: var(--ink);
			}
		}
	}
</style>
