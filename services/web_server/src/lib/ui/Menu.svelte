<script>
	import { scale } from 'svelte/transition';
	import Icon from './Icon.svelte';
	import { menuState, closeMenu } from './menu.js';

	let el = $state();
	let pos = $state({ x: 0, y: 0 });

	$effect(() => {
		if ($menuState.open && el) {
			const { innerWidth, innerHeight } = window;
			const r = el.getBoundingClientRect();
			pos = {
				x: Math.min($menuState.x, innerWidth - r.width - 8),
				y: Math.min($menuState.y, innerHeight - r.height - 8)
			};
		} else {
			pos = { x: $menuState.x, y: $menuState.y };
		}
	});

	function run(item) {
		if (item.disabled) return;
		closeMenu();
		item.action?.();
	}

	function onkeydown(e) {
		if (e.key === 'Escape') closeMenu();
	}
</script>

<svelte:window
	{onkeydown}
	onclick={() => $menuState.open && closeMenu()}
	oncontextmenu={(e) => {
		if ($menuState.open) {
			e.preventDefault();
			closeMenu();
		}
	}}
	onresize={() => $menuState.open && closeMenu()}
/>

{#if $menuState.open}
	<div
		bind:this={el}
		class="menu"
		style="left: {pos.x}px; top: {pos.y}px"
		transition:scale={{ duration: 110, start: 0.96 }}
		role="menu"
	>
		{#each $menuState.items as item, i (i)}
			{#if item.divider}
				<div class="divider" role="separator"></div>
			{:else}
				<button
					type="button"
					role="menuitem"
					class="item"
					class:danger={item.danger}
					disabled={item.disabled}
					onclick={(e) => {
						e.stopPropagation();
						run(item);
					}}
				>
					{#if item.icon}<Icon icon={item.icon} width={15} />{/if}
					<span>{item.label}</span>
				</button>
			{/if}
		{/each}
	</div>
{/if}

<style lang="scss">
	.menu {
		position: fixed;
		z-index: 1100;
		min-width: 180px;
		background: var(--raised);
		border: 1px solid var(--edge);
		border-radius: var(--radius-md);
		box-shadow: var(--shadow-overlay);
		padding: var(--space-1);
		display: flex;
		flex-direction: column;
		transform-origin: top left;
	}

	.item {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		width: 100%;
		background: none;
		border: none;
		border-radius: var(--radius-sm);
		padding: 0.5rem 0.65rem;
		color: var(--ink);
		font-family: inherit;
		font-size: var(--fs-sm);
		text-align: left;
		cursor: pointer;
		transition:
			background var(--dur-fast) var(--ease),
			color var(--dur-fast) var(--ease);

		:global(.iconify) {
			color: var(--ink-faint);
			flex-shrink: 0;
		}

		&:hover:not(:disabled) {
			background: var(--tint-soft);
		}
		&.danger {
			color: var(--danger);
			:global(.iconify) {
				color: var(--danger);
			}
			&:hover:not(:disabled) {
				background: var(--danger-soft);
			}
		}
		&:disabled {
			opacity: 0.45;
			cursor: not-allowed;
		}
	}

	.divider {
		height: 1px;
		background: var(--edge);
		margin: var(--space-1) 0;
	}
</style>
