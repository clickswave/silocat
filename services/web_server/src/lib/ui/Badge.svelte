<script>
	import Icon from './Icon.svelte';

	let {
		tone = 'neutral', // neutral | accent | ok | warn | danger
		icon = undefined,
		mono = false,
		children
	} = $props();
</script>

<span class="badge {tone}" class:mono>
	{#if icon}<Icon {icon} width={12} />{/if}
	{@render children?.()}
</span>

<style lang="scss">
	.badge {
		display: inline-flex;
		align-items: center;
		gap: var(--space-1);
		padding: 0.2rem 0.55rem;
		border-radius: var(--radius-full);
		font-size: var(--fs-xs);
		font-weight: var(--fw-medium);
		line-height: 1.4;
		white-space: nowrap;

		&.mono {
			font-family: var(--font-mono);
		}

		&.neutral {
			background: var(--tint-soft);
			border: 1px solid var(--edge);
			color: var(--ink-mute);
		}
		&.accent {
			background: var(--accent-soft);
			color: var(--accent);
		}
		&.ok {
			background: var(--ok-soft);
			color: var(--ok);
		}
		&.warn {
			background: var(--warn-soft);
			color: var(--warn);
		}
		&.danger {
			background: var(--danger-soft);
			color: var(--danger);
		}
	}
</style>
