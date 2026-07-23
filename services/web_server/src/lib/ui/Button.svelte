<script>
	import Spinner from './Spinner.svelte';

	let {
		variant = 'solid', // solid | ghost | quiet | danger | danger-solid
		size = 'md', // sm | md | lg
		type = 'button',
		href = undefined,
		disabled = false,
		loading = false,
		block = false,
		onclick = undefined,
		children,
		...rest
	} = $props();
</script>

{#if href && !disabled}
	<a {href} class="button {variant} {size}" class:block {onclick} {...rest}>
		{@render children?.()}
	</a>
{:else}
	<button {type} class="button {variant} {size}" class:block disabled={disabled || loading} {onclick} {...rest}>
		{#if loading}
			<Spinner size={size === 'sm' ? 13 : 15} />
		{/if}
		{@render children?.()}
	</button>
{/if}

<style lang="scss">
	.button {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: var(--space-2);
		border: 1px solid transparent;
		border-radius: var(--radius-md);
		font-family: inherit;
		font-weight: var(--fw-medium);
		line-height: 1;
		cursor: pointer;
		white-space: nowrap;
		text-decoration: none;
		transition:
			background var(--dur) var(--ease),
			border-color var(--dur) var(--ease),
			color var(--dur) var(--ease),
			opacity var(--dur) var(--ease);

		&:active:not(:disabled) {
			transform: translateY(1px);
		}
		&:disabled {
			opacity: 0.5;
			cursor: not-allowed;
		}

		/* sizes */
		&.sm {
			padding: 0.4rem 0.75rem;
			font-size: var(--fs-sm);
		}
		&.md {
			padding: 0.6rem 1.1rem;
			font-size: var(--fs-body);
		}
		&.lg {
			padding: 0.8rem 1.5rem;
			font-size: var(--fs-body);
		}
		&.block {
			width: 100%;
		}

		/* variants */
		&.solid {
			background: var(--accent);
			color: #fff;
			&:hover:not(:disabled) {
				background: var(--accent-hover);
			}
			&:active:not(:disabled) {
				background: var(--accent-press);
			}
		}
		&.ghost {
			background: transparent;
			border-color: var(--edge);
			color: var(--ink);
			&:hover:not(:disabled) {
				background: var(--tint-soft);
				border-color: var(--edge-strong);
			}
		}
		&.quiet {
			background: transparent;
			color: var(--ink-mute);
			&:hover:not(:disabled) {
				color: var(--ink);
				background: var(--tint-soft);
			}
		}
		&.danger {
			background: transparent;
			border-color: var(--edge);
			color: var(--danger);
			&:hover:not(:disabled) {
				border-color: var(--danger);
				background: var(--danger-soft);
			}
		}
		&.danger-solid {
			background: var(--danger);
			color: #fff;
			&:hover:not(:disabled) {
				opacity: 0.9;
			}
		}
	}
</style>
