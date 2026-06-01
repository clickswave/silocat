<script>
	let { version } = $props();
</script>

<div>{version}</div>
<style>
	div {
		position: absolute;
		bottom: 0.5rem;
		right: 0.5rem;
		font-size: 0.75rem;
		color: rgba(255, 255, 255, 0.45);
		background: var(--color-bg-accent);
		padding: 0.25rem 0.5rem;
		border-radius: 0.25rem;
		box-shadow: var(--shadow-sm);
		z-index: 1000;
	}
</style>
