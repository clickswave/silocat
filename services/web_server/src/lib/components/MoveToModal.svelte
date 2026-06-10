<script>
	import Icon from '@iconify/svelte';
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

<div
	class="mv-backdrop"
	transition:fade={{ duration: 150 }}
	role="presentation"
	onclick={(e) => {
		if (e.target === e.currentTarget) onclose();
	}}
>
	<div class="mv-shell" transition:scale={{ duration: 180, start: 0.96 }}>
		<header class="mv-head">
			<div class="title"><Icon icon="ri:folder-transfer-line" width="20" /><span>Move to</span></div>
			<button class="hbtn" onclick={() => onclose()} aria-label="Close">
				<Icon icon="ri:close-line" width="20" />
			</button>
		</header>

		<div class="mv-crumbs">
			{#each path as c, i}
				{#if i > 0}<span class="sep">/</span>{/if}
				<button class="crumb" class:active={i === path.length - 1} onclick={() => jump(i)}>
					{c.name}
				</button>
			{/each}
		</div>

		<div class="mv-list">
			{#if loading}
				<div class="mv-empty"><Icon icon="ri:loader-4-line" class="spin" width="24" /></div>
			{:else if folders.length === 0}
				<div class="mv-empty">No sub-folders here</div>
			{:else}
				{#each folders as f (f.id)}
					{@const blocked = excludeFolderIds.includes(f.id)}
					<button class="mv-row" disabled={blocked} onclick={() => open(f)}>
						<Icon icon="ri:folder-3-fill" width="20" />
						<span class="nm">{f.name}</span>
						{#if blocked}
							<span class="tag">moving</span>
						{:else}
							<Icon icon="ri:arrow-right-s-line" width="18" />
						{/if}
					</button>
				{/each}
			{/if}
		</div>

		<footer class="mv-foot">
			<span class="dest">Destination: <strong>{path[path.length - 1].name}</strong></span>
			<div class="foot-btns">
				<button class="btn ghost" onclick={() => onclose()}>Cancel</button>
				<button class="btn primary" onclick={() => onmove(currentId)}>Move here</button>
			</div>
		</footer>
	</div>
</div>

<style lang="scss">
	.mv-backdrop {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.6);
		backdrop-filter: blur(8px);
		z-index: 1100;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: var(--space-5);
	}
	.mv-shell {
		background: var(--bg-elevated);
		border: 1px solid var(--border-default);
		border-radius: var(--radius-lg);
		box-shadow: var(--shadow-lg);
		width: 100%;
		max-width: 440px;
		max-height: 80vh;
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}
	.mv-head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: var(--space-4) var(--space-5);
		.title {
			display: flex;
			align-items: center;
			gap: var(--space-2);
			font-weight: var(--fw-semibold);
			color: var(--text-primary);
		}
		.hbtn {
			background: transparent;
			border: none;
			color: var(--text-muted);
			cursor: pointer;
			display: flex;
			padding: 4px;
			border-radius: var(--radius-sm);
			&:hover {
				color: var(--text-primary);
				background: var(--tint-soft);
			}
		}
	}
	.mv-crumbs {
		display: flex;
		align-items: center;
		flex-wrap: wrap;
		gap: 2px;
		padding: 0 var(--space-5) var(--space-3);
		font-size: var(--fs-sm);
		.sep {
			color: var(--text-dim);
		}
		.crumb {
			background: none;
			border: none;
			color: var(--text-muted);
			cursor: pointer;
			padding: 2px 4px;
			border-radius: var(--radius-sm);
			&:hover {
				color: var(--text-primary);
			}
			&.active {
				color: var(--text-primary);
				font-weight: var(--fw-medium);
			}
		}
	}
	.mv-list {
		flex: 1;
		overflow-y: auto;
		padding: 0 var(--space-4) var(--space-3);
		display: flex;
		flex-direction: column;
		gap: 2px;
		min-height: 120px;

		.mv-empty {
			display: flex;
			align-items: center;
			justify-content: center;
			color: var(--text-muted);
			padding: var(--space-6);
			font-size: var(--fs-sm);
			:global(.spin) {
				animation: spin 1s linear infinite;
			}
		}
		.mv-row {
			display: flex;
			align-items: center;
			gap: var(--space-3);
			padding: var(--space-3);
			background: transparent;
			border: none;
			border-radius: var(--radius-sm);
			color: var(--text-secondary);
			cursor: pointer;
			text-align: left;
			width: 100%;
			transition: background var(--dur) var(--ease);
			.nm {
				flex: 1;
				overflow: hidden;
				text-overflow: ellipsis;
				white-space: nowrap;
				color: var(--text-primary);
			}
			.tag {
				font-size: var(--fs-xs);
				color: var(--text-dim);
			}
			&:hover:not(:disabled) {
				background: var(--tint-soft);
			}
			&:disabled {
				opacity: 0.45;
				cursor: not-allowed;
			}
		}
	}
	.mv-foot {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-3);
		padding: var(--space-4) var(--space-5);
		border-top: 1px solid var(--hairline);
		.dest {
			font-size: var(--fs-sm);
			color: var(--text-muted);
			overflow: hidden;
			text-overflow: ellipsis;
			white-space: nowrap;
			strong {
				color: var(--text-primary);
			}
		}
		.foot-btns {
			display: flex;
			gap: var(--space-2);
			flex: none;
		}
		.btn {
			border-radius: var(--radius-sm);
			padding: var(--space-2) var(--space-4);
			font-weight: var(--fw-medium);
			cursor: pointer;
			border: 1px solid transparent;
			font-family: inherit;
			&.ghost {
				background: transparent;
				border-color: var(--border-default);
				color: var(--text-secondary);
				&:hover {
					color: var(--text-primary);
				}
			}
			&.primary {
				background: var(--primary);
				color: #fff;
				&:hover {
					filter: brightness(1.05);
				}
			}
		}
	}
	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
	}

	@media (max-width: 600px) {
		.mv-backdrop {
			align-items: flex-end;
			padding: 0;
		}
		.mv-shell {
			max-width: 100%;
			max-height: 88vh;
			border-radius: var(--radius-lg) var(--radius-lg) 0 0;
		}
		.mv-foot .btn {
			flex: 1;
		}
	}
</style>
