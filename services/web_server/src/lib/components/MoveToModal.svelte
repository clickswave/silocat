<script>
	import Modal from '$lib/ui/Modal.svelte';
	import Icon from '$lib/ui/Icon.svelte';
	import { onMount } from 'svelte';
	import { fade, scale } from 'svelte/transition';
	import axios from 'axios';

	// excludeFolderIds: folder ids that are part of the move set (cannot be a target)
	let { excludeFolderIds = [], onmove = () => {}, onclose = () => {} } = $props();

	let path = $state([{ id: null, name: 'Home' }]);
	let folders = $state([]);
	let loading = $state(false);

	let currentId = $derived(path[path.length - 1].id);

	async function loadFolders(parentId) {
		loading = true;
		try {
			const { data } = await axios.post('/api/v1/sanctum/folder/list', { parent_id: parentId });
			folders = data?.data?.folders || [];
		} catch (e) {
			console.error('[move] list folders', e);
			folders = [];
		} finally {
			loading = false;
		}
	}

	function open(folder) {
		path = [...path, { id: folder.id, name: folder.name }];
		loadFolders(folder.id);
	}

	function jump(index) {
		path = path.slice(0, index + 1);
		loadFolders(path[index].id);
	}

	onMount(() => loadFolders(null));
</script>

<Modal open={true} title="Move to" icon="folder-move" size="sm" onclose={() => onclose()}>
	<div class="crumbs">
		{#each path as c, i (c.id ?? 'root')}
			{#if i > 0}<span class="sep">/</span>{/if}
			{#if i === path.length - 1}
				<span class="crumb current">{c.name}</span>
			{:else}
				<button type="button" class="crumb" onclick={() => jump(i)}>{c.name}</button>
			{/if}
		{/each}
	</div>

	<div class="targets">
		{#if loading}
			<div class="state"><Icon name="spinner" size={20} /></div>
		{:else if folders.length === 0}
			<div class="state">No sub-folders here</div>
		{:else}
			{#each folders as f (f.id)}
				{@const blocked = excludeFolderIds.includes(f.id)}
				<button type="button" class="target" disabled={blocked} onclick={() => open(f)}>
					<Icon name="folder" size={16} />
					<span class="t-name">{f.name}</span>
					{#if blocked}
						<span class="t-tag">moving</span>
					{:else}
						<Icon name="chevron-right" size={15} />
					{/if}
				</button>
			{/each}
		{/if}
	</div>

	<span class="dest">
		Destination: <span class="dest-name">{path[path.length - 1].name}</span>
	</span>

	{#snippet footer()}
		<button type="button" class="ghost" onclick={() => onclose()}>Cancel</button>
		<button type="button" class="primary" onclick={() => onmove(currentId)}>Move here</button>
	{/snippet}
</Modal>

<style lang="scss">
	.crumbs {
		display: flex;
		align-items: center;
		gap: var(--space-1);
		font-size: var(--fs-sm);
		color: var(--ink-mute);
		flex-wrap: wrap;
	}

	.crumb {
		padding: 0.25rem 0.375rem;
		border: 0;
		background: none;
		border-radius: var(--radius-sm);
		font: inherit;
		font-size: var(--fs-sm);
		color: var(--ink-mute);
		cursor: pointer;
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
		}
	}

	.sep {
		color: var(--ink-faint);
	}

	.targets {
		border: 1px solid var(--edge);
		border-radius: var(--radius-md);
		overflow: hidden;
		max-height: 240px;
		overflow-y: auto;
	}

	.target {
		width: 100%;
		display: flex;
		align-items: center;
		gap: 0.625rem;
		padding: 0.625rem 0.75rem;
		border: 0;
		border-bottom: 1px solid var(--edge);
		background: none;
		text-align: left;
		font: inherit;
		color: var(--ink-mute);
		cursor: pointer;
		transition: background var(--dur-fast) var(--ease);

		&:last-child {
			border-bottom: 0;
		}
		&:hover:not(:disabled) {
			background: var(--surface-hover);
		}
		&:disabled {
			opacity: 0.45;
			cursor: not-allowed;
		}
	}

	.t-name {
		flex: 1;
		min-width: 0;
		font-size: var(--fs-sm);
		color: var(--ink);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.t-tag {
		font-family: var(--font-mono);
		font-size: 0.6875rem;
		color: var(--ink-faint);
	}

	.state {
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 1.75rem 1rem;
		font-size: var(--fs-sm);
		color: var(--ink-faint);
	}

	.dest {
		font-size: var(--fs-sm);
		color: var(--ink-mute);
	}

	.dest-name {
		color: var(--ink);
		font-weight: var(--fw-medium);
	}

	.ghost,
	.primary {
		height: 34px;
		padding-inline: 0.875rem;
		border-radius: var(--radius-md);
		border: 1px solid transparent;
		font: inherit;
		font-size: var(--fs-sm);
		font-weight: var(--fw-medium);
		cursor: pointer;
		transition:
			background var(--dur-fast) var(--ease),
			color var(--dur-fast) var(--ease),
			filter var(--dur-fast) var(--ease);
	}

	.ghost {
		background: none;
		color: var(--ink-mute);

		&:hover {
			background: var(--tint-soft);
			color: var(--ink);
		}
	}

	.primary {
		background: var(--accent);
		color: #fff;

		&:hover {
			filter: brightness(1.08);
		}
	}
</style>
