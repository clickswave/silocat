<script>
	let { size = 16 } = $props();
</script>

<svg
	class="spinner"
	width={size}
	height={size}
	viewBox="0 0 24 24"
	fill="none"
	xmlns="http://www.w3.org/2000/svg"
	aria-hidden="true"
>
	<circle cx="12" cy="12" r="9.5" stroke="currentColor" stroke-opacity="0.25" stroke-width="3" />
	<path
		d="M21.5 12a9.5 9.5 0 0 0-9.5-9.5"
		stroke="currentColor"
		stroke-width="3"
		stroke-linecap="round"
	/>
</svg>

<style lang="scss">
	.spinner {
		animation: rotate 0.7s linear infinite;
		flex-shrink: 0;
	}
	@keyframes rotate {
		to {
			transform: rotate(360deg);
		}
	}
</style>
