<script>
	let {
		padding = 'md', // none | sm | md | lg
		hover = false,
		children,
		...rest
	} = $props();
</script>

<div class="ui-card pad-{padding}" class:hover {...rest}>
	{@render children?.()}
</div>

<style lang="scss">
	.ui-card {
		background: var(--surface);
		border: 1px solid var(--edge);
		border-radius: var(--radius-md);

		&.pad-sm {
			padding: var(--space-3);
		}
		&.pad-md {
			padding: var(--space-5);
		}
		&.pad-lg {
			padding: var(--space-6);
		}

		&.hover {
			transition:
				background var(--dur) var(--ease),
				border-color var(--dur) var(--ease);
			&:hover {
				background: var(--surface-hover);
				border-color: var(--edge-strong);
			}
		}
	}
</style>
