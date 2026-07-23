<script>
	let { width = '100%', height = '1rem', radius = 'var(--radius-sm)' } = $props();
</script>

<div class="skeleton" style="width: {width}; height: {height}; border-radius: {radius}" aria-hidden="true"></div>

<style lang="scss">
	.skeleton {
		background: var(--tint-softer);
		animation: pulse 1.4s var(--ease) infinite;
	}
	@keyframes pulse {
		0%,
		100% {
			opacity: 1;
		}
		50% {
			opacity: 0.45;
		}
	}
</style>
