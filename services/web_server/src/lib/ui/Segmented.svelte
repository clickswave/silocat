<script>
	import Icon from '@iconify/svelte';

	let {
		options = [], // [{ value, label?, icon? }]
		value = $bindable(),
		size = 'md', // sm | md
		onchange = undefined
	} = $props();

	function pick(v) {
		if (v === value) return;
		value = v;
		onchange?.(v);
	}
</script>

<div class="segmented {size}" role="tablist">
	{#each options as opt (opt.value)}
		<button
			type="button"
			role="tab"
			class="seg"
			class:active={value === opt.value}
			aria-selected={value === opt.value}
			title={opt.title || opt.label}
			onclick={() => pick(opt.value)}
		>
			{#if opt.icon}<Icon icon={opt.icon} width={size === 'sm' ? 14 : 15} />{/if}
			{#if opt.label}<span>{opt.label}</span>{/if}
		</button>
	{/each}
</div>

<style lang="scss">
	.segmented {
		display: inline-flex;
		align-items: center;
		gap: 2px;
		background: var(--tint-soft);
		border: 1px solid var(--edge);
		border-radius: var(--radius-sm);
		padding: 2px;
	}

	.seg {
		display: inline-flex;
		align-items: center;
		gap: var(--space-1);
		background: transparent;
		border: none;
		border-radius: calc(var(--radius-sm) - 2px);
		color: var(--ink-mute);
		font-family: inherit;
		font-weight: var(--fw-medium);
		cursor: pointer;
		transition:
			background var(--dur) var(--ease),
			color var(--dur) var(--ease);

		.segmented.sm & {
			padding: 0.25rem 0.55rem;
			font-size: var(--fs-xs);
		}
		.segmented.md & {
			padding: 0.35rem 0.75rem;
			font-size: var(--fs-sm);
		}

		&:hover {
			color: var(--ink);
		}
		&.active {
			background: var(--surface);
			color: var(--ink);
			box-shadow: inset 0 0 0 1px var(--edge);
		}
	}
</style>
