<script>
	let { version } = $props();
</script>

<div>{version}</div>
<style>
	div {
		position: absolute;
		bottom: var(--space-2);
		right: var(--space-2);
		font-family: var(--font-mono);
		font-size: var(--fs-xs);
		color: var(--text-muted);
		background: var(--tint-soft);
		border: 1px solid var(--border-default);
		padding: var(--space-1) var(--space-2);
		border-radius: var(--radius-sm);
		box-shadow: var(--shadow-card);
		z-index: 1000;
	}
</style>
