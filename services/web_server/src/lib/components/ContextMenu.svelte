<script>
	import Icon from '@iconify/svelte';
	import { tick } from 'svelte';

	// items: array of { label, icon, danger?, divider?, disabled? , action }
	let { x = 0, y = 0, items = [], onclose = () => {} } = $props();

	let menuEl = $state(null);
	let pos = $state({ left: x, top: y });

	async function place() {
		await tick();
		if (!menuEl) return;
		const r = menuEl.getBoundingClientRect();
		const pad = 8;
		let left = x;
		let top = y;
		if (left + r.width + pad > window.innerWidth) left = window.innerWidth - r.width - pad;
		if (top + r.height + pad > window.innerHeight) top = window.innerHeight - r.height - pad;
		pos = { left: Math.max(pad, left), top: Math.max(pad, top) };
	}

	$effect(() => {
		// re-place whenever coordinates change
		x;
		y;
		place();
	});

	function handleWindowClick() {
		onclose();
	}
</script>

<svelte:window onclick={handleWindowClick} oncontextmenu={handleWindowClick} onresize={() => onclose()} />

<div
	class="ctx-menu"
	bind:this={menuEl}
	style="left:{pos.left}px; top:{pos.top}px;"
	role="menu"
	tabindex="-1"
	onclick={(e) => e.stopPropagation()}
	oncontextmenu={(e) => e.preventDefault()}
>
	{#each items as it}
		{#if it.divider}
			<div class="ctx-divider"></div>
		{:else}
			<button
				class="ctx-item {it.danger ? 'danger' : ''}"
				disabled={it.disabled}
				role="menuitem"
				onclick={() => {
					onclose();
					it.action?.();
				}}
			>
				{#if it.icon}<Icon icon={it.icon} width="16" />{/if}
				<span>{it.label}</span>
			</button>
		{/if}
	{/each}
</div>

<style lang="scss">
	.ctx-menu {
		position: fixed;
		z-index: 1200;
		background: var(--bg-elevated);
		border: 1px solid var(--border-default);
		border-radius: var(--radius-md);
		padding: var(--space-1);
		min-width: 180px;
		box-shadow: var(--shadow-lg);
		display: flex;
		flex-direction: column;
		gap: 2px;
		animation: ctx-in 0.12s var(--ease);
	}
	@keyframes ctx-in {
		from {
			opacity: 0;
			transform: scale(0.97);
		}
		to {
			opacity: 1;
			transform: scale(1);
		}
	}
	.ctx-divider {
		height: 1px;
		background: var(--hairline);
		margin: 4px 6px;
	}
	.ctx-item {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		padding: var(--space-2) var(--space-3);
		background: transparent;
		border: none;
		color: var(--text-secondary);
		font-family: inherit;
		font-size: var(--fs-sm);
		font-weight: var(--fw-medium);
		cursor: pointer;
		border-radius: var(--radius-sm);
		text-align: left;
		width: 100%;
		transition: background var(--dur) var(--ease), color var(--dur) var(--ease);
		&:hover:not(:disabled) {
			background: var(--tint-soft);
			color: var(--text-primary);
		}
		&:disabled {
			opacity: 0.4;
			cursor: not-allowed;
		}
		&.danger {
			color: var(--danger);
			&:hover:not(:disabled) {
				background: rgba(255, 70, 85, 0.1);
			}
		}
	}
</style>
