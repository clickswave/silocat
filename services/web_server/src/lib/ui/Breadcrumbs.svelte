<script>
	import Icon from '@iconify/svelte';

	let {
		items = [], // [{ label, onclick? }] last item = current
		onnavigate = undefined // fallback handler receiving index
	} = $props();
</script>

<nav class="crumbs" aria-label="Breadcrumb">
	{#each items as item, i (i)}
		{#if i > 0}
			<Icon icon="ri:arrow-right-s-line" width={14} class="sep" />
		{/if}
		{#if i < items.length - 1}
			<button type="button" class="crumb" onclick={() => (item.onclick ? item.onclick() : onnavigate?.(i))}>
				{item.label}
			</button>
		{:else}
			<span class="crumb current" aria-current="page">{item.label}</span>
		{/if}
	{/each}
</nav>

<style lang="scss">
	.crumbs {
		display: flex;
		align-items: center;
		gap: var(--space-1);
		min-width: 0;
		color: var(--ink-faint);

		:global(.sep) {
			color: var(--ink-faint);
			flex-shrink: 0;
		}
	}

	.crumb {
		background: none;
		border: none;
		padding: 0.15rem 0.3rem;
		border-radius: var(--radius-sm);
		font-family: inherit;
		font-size: var(--fs-sm);
		color: var(--ink-mute);
		cursor: pointer;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		max-width: 200px;
		transition:
			background var(--dur-fast) var(--ease),
			color var(--dur-fast) var(--ease);

		&:hover {
			background: var(--tint-soft);
			color: var(--ink);
		}

		&.current {
			color: var(--ink);
			font-weight: var(--fw-medium);
			cursor: default;
			&:hover {
				background: none;
			}
		}
	}
</style>
