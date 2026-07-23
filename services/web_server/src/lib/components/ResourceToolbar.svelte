<script>
	import Icon from '@iconify/svelte';

	let {
		search = $bindable(''),
		sortKey = $bindable('name'),
		sortDir = $bindable('asc'),
		view = $bindable('grid'),
		placeholder = 'Search',
		sorts = [
			{ key: 'name', label: 'Name' },
			{ key: 'size', label: 'Size' },
			{ key: 'date', label: 'Date' }
		]
	} = $props();

	function toggleSort(key) {
		if (sortKey === key) sortDir = sortDir === 'asc' ? 'desc' : 'asc';
		else {
			sortKey = key;
			sortDir = key === 'date' ? 'desc' : 'asc';
		}
	}
</script>

<div class="toolbar">
	<div class="search-box">
		<Icon icon="ri:search-line" width="17" />
		<input type="text" {placeholder} bind:value={search} spellcheck="false" />
		{#if search}
			<button class="clear" aria-label="Clear search" onclick={() => (search = '')}>
				<Icon icon="ri:close-line" width="15" />
			</button>
		{/if}
	</div>

	<div class="toolbar-right">
		<div class="sort-group">
			{#each sorts as s (s.key)}
				<button class="sort-btn" class:active={sortKey === s.key} onclick={() => toggleSort(s.key)}>
					{s.label}
					{#if sortKey === s.key}
						<Icon icon={sortDir === 'asc' ? 'ri:arrow-up-s-line' : 'ri:arrow-down-s-line'} width="15" />
					{/if}
				</button>
			{/each}
		</div>

		<div class="view-toggle">
			<button class:active={view === 'grid'} aria-label="Grid view" title="Grid view" onclick={() => (view = 'grid')}>
				<Icon icon="ri:layout-grid-line" width="17" />
			</button>
			<button class:active={view === 'list'} aria-label="List view" title="List view" onclick={() => (view = 'list')}>
				<Icon icon="ri:list-unordered" width="17" />
			</button>
		</div>
	</div>
</div>

<style lang="scss">
	.toolbar {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		flex-wrap: wrap;
	}

	.search-box {
		flex: 1;
		min-width: 200px;
		display: flex;
		align-items: center;
		gap: var(--space-2);
		background: var(--bg);
		border: 1px solid var(--edge);
		border-radius: var(--radius-md);
		padding: 0 var(--space-3);
		color: var(--ink-faint);
		transition:
			border-color var(--dur) var(--ease),
			box-shadow var(--dur) var(--ease);

		&:focus-within {
			border-color: var(--accent);
			box-shadow: 0 0 0 3px var(--focus-ring);
		}
		input {
			flex: 1;
			min-width: 0;
			background: transparent;
			border: none;
			outline: none;
			color: var(--ink);
			font-family: inherit;
			font-size: var(--fs-sm);
			padding: 0.55rem 0;
		}
		.clear {
			display: flex;
			background: none;
			border: none;
			color: var(--ink-faint);
			cursor: pointer;
			padding: 2px;
			border-radius: var(--radius-sm);
			&:hover {
				color: var(--ink);
			}
		}
	}

	.toolbar-right {
		display: flex;
		align-items: center;
		gap: var(--space-2);
	}

	.sort-group {
		display: flex;
		background: var(--tint-soft);
		border: 1px solid var(--edge);
		border-radius: var(--radius-sm);
		padding: 2px;
		gap: 2px;

		.sort-btn {
			display: inline-flex;
			align-items: center;
			gap: 2px;
			background: transparent;
			border: none;
			color: var(--ink-mute);
			font-family: inherit;
			font-size: var(--fs-sm);
			font-weight: var(--fw-medium);
			padding: 0.35rem 0.7rem;
			border-radius: calc(var(--radius-sm) - 2px);
			cursor: pointer;
			transition:
				background var(--dur) var(--ease),
				color var(--dur) var(--ease);

			&:hover {
				color: var(--ink);
			}
			&.active {
				background: var(--surface);
				color: var(--ink);
				box-shadow: inset 0 0 0 1px var(--edge);
			}
		}
	}

	.view-toggle {
		display: flex;
		border: 1px solid var(--edge);
		border-radius: var(--radius-sm);
		overflow: hidden;

		button {
			display: flex;
			align-items: center;
			justify-content: center;
			background: var(--tint-soft);
			border: none;
			color: var(--ink-mute);
			cursor: pointer;
			padding: 0.4rem 0.55rem;
			transition:
				background var(--dur) var(--ease),
				color var(--dur) var(--ease);

			&:hover {
				color: var(--ink);
			}
			&.active {
				background: var(--accent-soft);
				color: var(--accent);
			}
		}
	}

	@media (max-width: 620px) {
		.search-box {
			order: -1;
			flex-basis: 100%;
		}
		.toolbar-right {
			width: 100%;
			justify-content: space-between;
		}
		.sort-group {
			flex: 1;
		}
	}
</style>
