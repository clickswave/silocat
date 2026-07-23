<script>
	import Icon from '@iconify/svelte';
	import FileCard from '$lib/components/FileCard.svelte';
	import FolderCard from '$lib/components/FolderCard.svelte';
	import ConfirmModal from '$lib/components/ConfirmModal.svelte';
	import ContextMenu from '$lib/components/ContextMenu.svelte';
	import { FrontendClient } from '$lib/frontendClient';
	import { onMount } from 'svelte';
	import { toast } from 'svelte-sonner';
	import { flip } from 'svelte/animate';
	import { fade } from 'svelte/transition';
	import { useQueryClient } from '@tanstack/svelte-query';

	const queryClient = useQueryClient();
	// Restore/permanent-delete change quota usage; refresh the shared storage
	// query so the sidebar + dashboard meters update without a manual reload.
	function refreshStorage() {
		queryClient.invalidateQueries({ queryKey: ['fetchStorageStats'] });
	}

	let trashItems = $state([]);
	let loading = $state(true);

	onMount(loadTrash);

	async function loadTrash() {
		loading = true;
		try {
			const folderRes = await FrontendClient.post('/api/v1/sanctum/folder/list', { deleted: true });
			const fileRes = await FrontendClient.post('/api/v1/sanctum/file/list', { deleted: true });

			let files = [];
			let folders = [];
			if (folderRes.data.status === 200) {
				folders = folderRes.data.data.folders.map((f) => ({ ...f, type: 'folder' }));
			}
			if (fileRes.data.status === 200) {
				files = fileRes.data.data.files.map((f) => ({ ...f, type: 'file' }));
			}
			trashItems = [...folders, ...files];
		} catch (e) {
			console.error(e);
			toast.error('Failed to load trash');
		} finally {
			loading = false;
		}
	}

	function formatSize(bytes) {
		if (!bytes) return '0 B';
		const k = 1024;
		const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
		const i = Math.min(Math.floor(Math.log(bytes) / Math.log(k)), sizes.length - 1);
		return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
	}
	function formatTime(dateString) {
		if (!dateString) return '—';
		const date = new Date(dateString);
		return date.toLocaleDateString();
	}
	function getFileType(mime) {
		if (!mime) return 'file';
		if (mime.includes('image')) return 'image';
		if (mime.includes('video')) return 'video';
		if (mime.includes('audio')) return 'audio';
		if (mime.includes('pdf') || mime.includes('document')) return 'doc';
		return 'file';
	}

	// ---- search / sort / view ----
	let searchQuery = $state('');
	let sortKey = $state('date');
	let sortDir = $state('desc');
	let viewMode = $state('grid');

	function matchesSearch(name) {
		const q = searchQuery.trim().toLowerCase();
		return !q || (name || '').toLowerCase().includes(q);
	}
	function applySort(list) {
		const dir = sortDir === 'asc' ? 1 : -1;
		return [...list].sort((a, b) => {
			if (sortKey === 'size') {
				const sa = a.type === 'folder' ? 0 : Number(a.size) || 0;
				const sb = b.type === 'folder' ? 0 : Number(b.size) || 0;
				if (sa !== sb) return dir * (sa - sb);
				return a.name.localeCompare(b.name);
			}
			if (sortKey === 'date') {
				const da = new Date(a.created_on || 0).getTime();
				const db = new Date(b.created_on || 0).getTime();
				if (da !== db) return dir * (da - db);
				return a.name.localeCompare(b.name);
			}
			return dir * a.name.localeCompare(b.name);
		});
	}
	function toggleSort(key) {
		if (sortKey === key) sortDir = sortDir === 'asc' ? 'desc' : 'asc';
		else {
			sortKey = key;
			sortDir = key === 'date' ? 'desc' : 'asc';
		}
	}

	let displayFolders = $derived(
		applySort(trashItems.filter((i) => i.type === 'folder' && matchesSearch(i.name)))
	);
	let displayFiles = $derived(
		applySort(trashItems.filter((i) => i.type === 'file' && matchesSearch(i.name)))
	);
	let orderedKeys = $derived([
		...displayFolders.map((f) => `folder:${f.id}`),
		...displayFiles.map((f) => `file:${f.id}`)
	]);
	let totalCount = $derived(displayFolders.length + displayFiles.length);

	// ---- selection ----
	let selected = $state(new Set());
	let lastKey = null;
	function keyOf(item) {
		return `${item.type}:${item.id}`;
	}
	function toggleKey(key) {
		const s = new Set(selected);
		s.has(key) ? s.delete(key) : s.add(key);
		selected = s;
	}
	function selectRange(toKey) {
		const a = orderedKeys.indexOf(lastKey);
		const b = orderedKeys.indexOf(toKey);
		if (a === -1 || b === -1) {
			selected = new Set([toKey]);
			return;
		}
		const [lo, hi] = a < b ? [a, b] : [b, a];
		selected = new Set(orderedKeys.slice(lo, hi + 1));
	}
	function clickItem(e, item) {
		const key = keyOf(item);
		if (e.shiftKey && lastKey) {
			selectRange(key);
			return;
		}
		if (e.ctrlKey || e.metaKey) {
			toggleKey(key);
			lastKey = key;
			return;
		}
		selected = new Set([key]);
		lastKey = key;
	}
	function selectAll() {
		selected = new Set(orderedKeys);
	}
	function clearSelection() {
		selected = new Set();
		lastKey = null;
	}
	function selectedList() {
		const map = new Map(trashItems.map((i) => [keyOf(i), i]));
		return [...selected].map((k) => map.get(k)).filter(Boolean);
	}

	// ---- single actions ----
	async function handleRestore(item) {
		try {
			const endpoint =
				item.type === 'folder' ? '/api/v1/sanctum/folder/restore' : '/api/v1/sanctum/file/restore';
			const payload = item.type === 'folder' ? { folder_id: item.id } : { file_id: item.id };
			await FrontendClient.post(endpoint, payload);
			toast.success('Restored ' + item.name);
			refreshStorage();
			selected.delete(keyOf(item));
			await loadTrash();
		} catch (e) {
			console.error(e);
			toast.error('Failed to restore item');
		}
	}

	let showDeleteModal = $state(false);
	let itemToDelete = $state(null);
	let deletedItemCount = $state(null);
	function handleDelete(item) {
		itemToDelete = item;
		deletedItemCount = null;
		if (item.type === 'folder') fetchDeletionStats(item.id);
		showDeleteModal = true;
	}
	async function fetchDeletionStats(folderId) {
		try {
			const res = await FrontendClient.post('/api/v1/sanctum/folder/stats', { folder_id: folderId });
			if (res.data?.data) deletedItemCount = res.data.data.total_items;
		} catch (e) {
			deletedItemCount = 'unknown';
		}
	}
	async function permanentDelete(item) {
		const endpoint =
			item.type === 'folder'
				? '/api/v1/sanctum/folder/permanent-delete'
				: '/api/v1/sanctum/file/permanent-delete';
		const payload = item.type === 'folder' ? { folder_id: item.id } : { file_id: item.id };
		await FrontendClient.post(endpoint, payload);
	}
	async function confirmDelete() {
		showDeleteModal = false;
		if (!itemToDelete) return;
		loading = true;
		try {
			await permanentDelete(itemToDelete);
			toast.success('Deleted ' + itemToDelete.name);
		refreshStorage();
			await loadTrash();
		} catch (e) {
			console.error(e);
			toast.error('Failed to delete item');
			loading = false;
		}
	}

	// ---- bulk actions ----
	async function bulkRestore() {
		const items = selectedList();
		if (!items.length) return;
		loading = true;
		let ok = 0;
		for (const item of items) {
			try {
				const endpoint =
					item.type === 'folder'
						? '/api/v1/sanctum/folder/restore'
						: '/api/v1/sanctum/file/restore';
				const payload = item.type === 'folder' ? { folder_id: item.id } : { file_id: item.id };
				await FrontendClient.post(endpoint, payload);
				ok++;
			} catch (e) {
				console.error('bulk restore', e);
			}
		}
		toast.success(`Restored ${ok} item${ok === 1 ? '' : 's'}`);
		refreshStorage();
		clearSelection();
		await loadTrash();
	}

	let showBulkDeleteModal = $state(false);
	function bulkDelete() {
		if (selected.size === 0) return;
		showBulkDeleteModal = true;
	}
	async function confirmBulkDelete() {
		showBulkDeleteModal = false;
		const items = selectedList();
		loading = true;
		let ok = 0;
		for (const item of items) {
			try {
				await permanentDelete(item);
				ok++;
			} catch (e) {
				console.error('bulk delete', e);
			}
		}
		toast.success(`Permanently deleted ${ok} item${ok === 1 ? '' : 's'}`);
		refreshStorage();
		clearSelection();
		await loadTrash();
	}

	// ---- empty trash ----
	let showEmptyModal = $state(false);
	let emptying = $state(false);
	function handleEmptyTrash() {
		if (trashItems.length === 0) {
			toast('Trash is already empty');
			return;
		}
		showEmptyModal = true;
	}
	async function confirmEmptyTrash() {
		showEmptyModal = false;
		emptying = true;
		loading = true;
		const items = [...trashItems];
		let ok = 0;
		for (const item of items) {
			try {
				await permanentDelete(item);
				ok++;
			} catch (e) {
				console.error('empty trash', e);
			}
		}
		emptying = false;
		toast.success(`Emptied trash (${ok} item${ok === 1 ? '' : 's'} permanently deleted)`);
		refreshStorage();
		clearSelection();
		await loadTrash();
	}

	// ---- context menu ----
	let ctx = $state({ open: false, x: 0, y: 0, items: [] });
	function closeCtx() {
		ctx = { ...ctx, open: false };
	}
	function openItemContext(e, item) {
		e.preventDefault();
		e.stopPropagation();
		const key = keyOf(item);
		if (!selected.has(key)) {
			selected = new Set([key]);
			lastKey = key;
		}
		const multi = selected.size > 1;
		const list = multi
			? [
					{ label: `Restore (${selected.size})`, icon: 'ri:arrow-go-back-line', action: bulkRestore },
					{ divider: true },
					{
						label: `Delete forever (${selected.size})`,
						icon: 'ri:delete-bin-line',
						danger: true,
						action: bulkDelete
					}
				]
			: [
					{ label: 'Restore', icon: 'ri:arrow-go-back-line', action: () => handleRestore(item) },
					{ divider: true },
					{
						label: 'Delete forever',
						icon: 'ri:delete-bin-line',
						danger: true,
						action: () => handleDelete(item)
					}
				];
		ctx = { open: true, x: e.clientX, y: e.clientY, items: list };
	}
	function openEmptyContext(e) {
		if (e.target.closest('[data-key]')) return;
		e.preventDefault();
		ctx = {
			open: true,
			x: e.clientX,
			y: e.clientY,
			items: [
				{ label: 'Select all', icon: 'ri:checkbox-multiple-line', disabled: totalCount === 0, action: selectAll },
				{ divider: true },
				{ label: 'Empty trash', icon: 'ri:delete-bin-7-line', danger: true, disabled: totalCount === 0, action: handleEmptyTrash }
			]
		};
	}

	function onWindowKeydown(e) {
		const tag = e.target?.tagName;
		if (tag === 'INPUT' || tag === 'TEXTAREA' || e.target?.isContentEditable) return;
		if (showDeleteModal || showBulkDeleteModal || showEmptyModal) return;
		if (e.key === 'Escape') {
			clearSelection();
			closeCtx();
		} else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'a') {
			if (totalCount) {
				e.preventDefault();
				selectAll();
			}
		} else if (e.key === 'Delete' || e.key === 'Backspace') {
			if (selected.size) {
				e.preventDefault();
				bulkDelete();
			}
		}
	}
</script>

<svelte:window onkeydown={onWindowKeydown} />

<div class="page-container">
	<header class="page-header">
		<div class="title-group">
			<h1>Trash</h1>
			<p>Items are kept here until you permanently delete them.</p>
		</div>
		<button
			class="empty-btn"
			onclick={handleEmptyTrash}
			disabled={totalCount === 0 || emptying}
			title="Permanently delete everything in trash"
		>
			<Icon icon="ri:delete-bin-7-line" width="18" />
			<span>Empty Trash</span>
		</button>
	</header>

	<!-- Toolbar -->
	<div class="files-toolbar">
		<div class="search-box">
			<Icon icon="ri:search-line" width="18" />
			<input type="text" placeholder="Search trash" bind:value={searchQuery} spellcheck="false" />
			{#if searchQuery}
				<button class="clear-search" aria-label="Clear" onclick={() => (searchQuery = '')}>
					<Icon icon="ri:close-line" width="16" />
				</button>
			{/if}
		</div>
		<div class="toolbar-right">
			<div class="sort-group">
				{#each [{ k: 'name', l: 'Name' }, { k: 'size', l: 'Size' }, { k: 'date', l: 'Deleted' }] as s}
					<button class="sort-btn {sortKey === s.k ? 'active' : ''}" onclick={() => toggleSort(s.k)}>
						{s.l}
						{#if sortKey === s.k}
							<Icon icon={sortDir === 'asc' ? 'ri:arrow-up-s-line' : 'ri:arrow-down-s-line'} width="16" />
						{/if}
					</button>
				{/each}
			</div>
			<div class="view-toggle">
				<button class={viewMode === 'grid' ? 'active' : ''} aria-label="Grid" onclick={() => (viewMode = 'grid')}>
					<Icon icon="ri:layout-grid-line" width="18" />
				</button>
				<button class={viewMode === 'list' ? 'active' : ''} aria-label="List" onclick={() => (viewMode = 'list')}>
					<Icon icon="ri:list-unordered" width="18" />
				</button>
			</div>
		</div>
	</div>

	<!-- Bulk bar -->
	{#if selected.size > 0}
		<div class="bulk-bar" transition:fade={{ duration: 120 }}>
			<div class="bulk-left">
				<button class="bulk-x" aria-label="Clear" onclick={clearSelection}>
					<Icon icon="ri:close-line" width="18" />
				</button>
				<span class="bulk-count">{selected.size} selected</span>
				<button class="bulk-link" onclick={selectAll} disabled={selected.size === totalCount}>Select all</button>
			</div>
			<div class="bulk-actions">
				<button class="bulk-btn" onclick={bulkRestore}>
					<Icon icon="ri:arrow-go-back-line" width="18" /><span>Restore</span>
				</button>
				<button class="bulk-btn danger" onclick={bulkDelete}>
					<Icon icon="ri:delete-bin-line" width="18" /><span>Delete forever</span>
				</button>
			</div>
		</div>
	{/if}

	<!-- Content -->
	<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
	<div class="resource-area {viewMode}" oncontextmenu={openEmptyContext} role="region" aria-label="Trash items">
		{#if loading}
			{#each Array(viewMode === 'grid' ? 8 : 6) as _, i (i)}
				<div class="skeleton {viewMode}"></div>
			{/each}
		{:else if totalCount === 0}
			<div class="empty-state">
				{#if searchQuery}
					<Icon icon="ri:search-eye-line" width="56" />
					<p>No items match “{searchQuery}”.</p>
					<button class="text-btn" onclick={() => (searchQuery = '')}>Clear search</button>
				{:else}
					<Icon icon="ri:delete-bin-line" width="56" />
					<p>Trash is empty</p>
				{/if}
			</div>
		{:else if viewMode === 'grid'}
			{#each displayFolders as item (item.id)}
				<div
					class="cell {selected.has(`folder:${item.id}`) ? 'is-selected' : ''}"
					data-key={`folder:${item.id}`}
					animate:flip={{ duration: 220 }}
					oncontextmenu={(e) => openItemContext(e, item)}
					role="listitem"
				>
					<FolderCard
						name={item.name}
						count={item.count || 0}
						starred={item.starred}
						isTrash={true}
						onclick={(e) => clickItem(e, item)}
						onrestore={() => handleRestore(item)}
						ondelete={() => handleDelete(item)}
					/>
				</div>
			{/each}
			{#each displayFiles as item (item.id)}
				<div
					class="cell {selected.has(`file:${item.id}`) ? 'is-selected' : ''}"
					data-key={`file:${item.id}`}
					animate:flip={{ duration: 220 }}
					oncontextmenu={(e) => openItemContext(e, item)}
					role="listitem"
				>
					<FileCard
						name={item.name}
						size={formatSize(item.size)}
						type={getFileType(item.mime)}
						starred={item.starred}
						isTrash={true}
						onclick={(e) => clickItem(e, item)}
						onrestore={() => handleRestore(item)}
						ondelete={() => handleDelete(item)}
					/>
				</div>
			{/each}
		{:else}
			<div class="list-head">
				<span>Name</span>
				<span>Size</span>
				<span>Deleted</span>
				<span></span>
			</div>
			{#each [...displayFolders, ...displayFiles] as item (item.type + item.id)}
				<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_noninteractive_element_interactions -->
				<div
					class="row {selected.has(keyOf(item)) ? 'is-selected' : ''}"
					data-key={keyOf(item)}
					animate:flip={{ duration: 220 }}
					oncontextmenu={(e) => openItemContext(e, item)}
					onclick={(e) => clickItem(e, item)}
					role="listitem"
				>
					<span class="r-name">
						<Icon
							icon={item.type === 'folder' ? 'ri:folder-3-fill' : getFileType(item.mime) === 'image' ? 'ri:image-2-fill' : getFileType(item.mime) === 'video' ? 'ri:film-fill' : getFileType(item.mime) === 'audio' ? 'ri:music-fill' : getFileType(item.mime) === 'doc' ? 'ri:file-text-fill' : 'ri:file-fill'}
							width="20"
							class="r-icon {item.type === 'folder' ? 'folder' : ''}"
						/>
						<span class="nm" title={item.name}>{item.name}</span>
					</span>
					<span class="r-size">{item.type === 'folder' ? (item.count || 0) + ' items' : formatSize(item.size)}</span>
					<span class="r-date">{formatTime(item.created_on)}</span>
					<span class="r-actions">
						<button class="r-act" title="Restore" aria-label="Restore" onclick={(e) => { e.stopPropagation(); handleRestore(item); }}>
							<Icon icon="ri:arrow-go-back-line" width="16" />
						</button>
						<button class="r-act danger" title="Delete forever" aria-label="Delete forever" onclick={(e) => { e.stopPropagation(); handleDelete(item); }}>
							<Icon icon="ri:delete-bin-line" width="16" />
						</button>
					</span>
				</div>
			{/each}
		{/if}
	</div>
</div>

{#if ctx.open}
	<ContextMenu x={ctx.x} y={ctx.y} items={ctx.items} onclose={closeCtx} />
{/if}

<ConfirmModal
	bind:show={showDeleteModal}
	title="Permanently delete?"
	icon="ri:delete-bin-7-line"
	message={`Permanently delete "${itemToDelete?.name}"? ${
		itemToDelete?.type === 'folder'
			? deletedItemCount !== null
				? deletedItemCount === 'unknown'
					? 'This cannot be undone.'
					: `This includes ${deletedItemCount} items inside and cannot be undone.`
				: '(Calculating items...)'
			: 'This cannot be undone.'
	}`}
	confirmLabel="Delete forever"
	isDanger={true}
	onconfirm={confirmDelete}
/>

<ConfirmModal
	bind:show={showBulkDeleteModal}
	title="Permanently delete selected?"
	icon="ri:delete-bin-7-line"
	message={`Permanently delete ${selected.size} selected item${selected.size === 1 ? '' : 's'}? This cannot be undone.`}
	confirmLabel="Delete forever"
	isDanger={true}
	onconfirm={confirmBulkDelete}
/>

<ConfirmModal
	bind:show={showEmptyModal}
	title="Empty trash?"
	icon="ri:alarm-warning-line"
	message={`This will permanently delete all ${trashItems.length} item${trashItems.length === 1 ? '' : 's'} in your trash. This is irreversible — files cannot be recovered afterwards.`}
	confirmLabel="Empty trash"
	isDanger={true}
	onconfirm={confirmEmptyTrash}
/>

<style lang="scss">
	.page-container {
		width: 100%;
		height: 100%;
		display: flex;
		flex-direction: column;
		gap: var(--space-5);
		color: var(--text-primary);
	}

	.page-header {
		display: flex;
		justify-content: space-between;
		align-items: flex-end;
		gap: var(--space-4);
		flex-wrap: wrap;

		.title-group {
			h1 {
				font-size: var(--fs-h3);
				font-weight: var(--fw-semibold);
				margin: 0 0 var(--space-1) 0;
			}
			p {
				color: var(--text-muted);
				margin: 0;
				font-size: var(--fs-sm);
			}
		}
		.empty-btn {
			display: inline-flex;
			align-items: center;
			gap: var(--space-2);
			background: transparent;
			border: 1px solid var(--danger);
			color: var(--danger);
			font-family: inherit;
			font-weight: var(--fw-medium);
			font-size: var(--fs-sm);
			padding: var(--space-2) var(--space-4);
			border-radius: var(--radius-pill, 999px);
			cursor: pointer;
			transition: background var(--dur) var(--ease), color var(--dur) var(--ease);
			&:hover:not(:disabled) {
				background: var(--danger);
				color: #fff;
			}
			&:disabled {
				opacity: 0.4;
				cursor: not-allowed;
			}
		}
	}

	/* Toolbar (shared visual language with /home/files) */
	.files-toolbar {
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
		background: var(--bg-input);
		border: 1px solid var(--border-default);
		border-radius: var(--radius-md);
		padding: 0 var(--space-3);
		color: var(--text-muted);
		&:focus-within {
			border-color: var(--primary);
			box-shadow: 0 0 0 3px var(--primary-glow);
		}
		input {
			flex: 1;
			min-width: 0;
			background: transparent;
			border: none;
			outline: none;
			color: var(--text-primary);
			font-family: inherit;
			font-size: var(--fs-sm);
			padding: 0.6rem 0;
		}
		.clear-search {
			background: none;
			border: none;
			color: var(--text-muted);
			cursor: pointer;
			display: flex;
			padding: 2px;
			&:hover {
				color: var(--text-primary);
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
		border: 1px solid var(--border-default);
		border-radius: var(--radius-md);
		padding: 2px;
		gap: 2px;
		.sort-btn {
			display: inline-flex;
			align-items: center;
			gap: 2px;
			background: transparent;
			border: none;
			color: var(--text-muted);
			font-family: inherit;
			font-size: var(--fs-sm);
			font-weight: var(--fw-medium);
			padding: var(--space-2) var(--space-3);
			border-radius: var(--radius-sm);
			cursor: pointer;
			&:hover {
				color: var(--text-primary);
			}
			&.active {
				background: var(--bg-elevated);
				color: var(--text-primary);
				box-shadow: var(--shadow-card);
			}
		}
	}
	.view-toggle {
		display: flex;
		border: 1px solid var(--border-default);
		border-radius: var(--radius-md);
		overflow: hidden;
		button {
			background: var(--tint-soft);
			border: none;
			color: var(--text-muted);
			cursor: pointer;
			display: flex;
			align-items: center;
			justify-content: center;
			padding: var(--space-2);
			&:hover {
				color: var(--text-primary);
			}
			&.active {
				background: var(--primary);
				color: #fff;
			}
		}
	}

	/* Bulk bar */
	.bulk-bar {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-3);
		flex-wrap: wrap;
		background: var(--bg-elevated);
		border: 1px solid var(--border-default);
		border-radius: var(--radius-md);
		padding: var(--space-2) var(--space-3);
		box-shadow: var(--shadow-card);
		.bulk-left {
			display: flex;
			align-items: center;
			gap: var(--space-3);
		}
		.bulk-x {
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
		.bulk-count {
			font-weight: var(--fw-semibold);
			color: var(--text-primary);
			font-size: var(--fs-sm);
		}
		.bulk-link {
			background: none;
			border: none;
			color: var(--primary);
			cursor: pointer;
			font-size: var(--fs-sm);
			font-weight: var(--fw-medium);
			&:disabled {
				color: var(--text-dim);
				cursor: default;
			}
		}
		.bulk-actions {
			display: flex;
			gap: var(--space-1);
			flex-wrap: wrap;
		}
		.bulk-btn {
			display: inline-flex;
			align-items: center;
			gap: var(--space-1);
			background: transparent;
			border: none;
			color: var(--text-secondary);
			font-family: inherit;
			font-size: var(--fs-sm);
			font-weight: var(--fw-medium);
			padding: var(--space-2) var(--space-3);
			border-radius: var(--radius-sm);
			cursor: pointer;
			&:hover {
				background: var(--tint-soft);
				color: var(--text-primary);
			}
			&.danger {
				color: var(--danger);
				&:hover {
					background: var(--danger-soft);
				}
			}
		}
	}

	/* Resource area */
	.resource-area {
		position: relative;
		min-height: 200px;
		padding-bottom: var(--space-6);
		&.grid {
			display: grid;
			grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
			grid-auto-rows: 1fr;
			gap: var(--space-5);
		}
		&.list {
			display: flex;
			flex-direction: column;
			gap: 2px;
		}
	}
	.cell {
		position: relative;
		border-radius: var(--radius-md);
		&.is-selected :global(.file-card),
		&.is-selected :global(.folder-card) {
			box-shadow: inset 0 0 0 2px var(--primary);
			border-color: var(--primary);
		}
	}

	.skeleton {
		border-radius: var(--radius-md);
		background: linear-gradient(100deg, var(--tint-soft) 30%, var(--tint-softer) 50%, var(--tint-soft) 70%);
		background-size: 200% 100%;
		animation: shimmer 1.3s infinite;
		&.grid {
			min-height: 160px;
		}
		&.list {
			height: 52px;
		}
	}
	@keyframes shimmer {
		from {
			background-position: 200% 0;
		}
		to {
			background-position: -200% 0;
		}
	}

	.empty-state {
		grid-column: 1 / -1;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		padding: var(--space-10) 0;
		gap: var(--space-4);
		color: var(--text-secondary);
		p {
			font-weight: var(--fw-medium);
			margin: 0;
		}
		.text-btn {
			background: none;
			border: none;
			color: var(--primary);
			cursor: pointer;
			font-weight: var(--fw-semibold);
			font-size: var(--fs-sm);
		}
	}

	/* List view */
	.list-head,
	.row {
		display: grid;
		grid-template-columns: 1fr 110px 130px 80px;
		align-items: center;
		gap: var(--space-3);
		padding: 0 var(--space-3);
	}
	.list-head {
		height: 36px;
		font-size: var(--fs-xs);
		text-transform: uppercase;
		letter-spacing: 0.04em;
		color: var(--text-dim);
		border-bottom: 1px solid var(--hairline);
	}
	.row {
		height: 52px;
		border-radius: var(--radius-sm);
		cursor: pointer;
		color: var(--text-secondary);
		user-select: none;
		transition: background var(--dur) var(--ease);
		&:hover {
			background: var(--bg-card-hover);
			.r-act {
				opacity: 1;
			}
		}
		&.is-selected {
			background: var(--tint-soft);
			box-shadow: inset 2px 0 0 var(--primary);
		}
		.r-name {
			display: flex;
			align-items: center;
			gap: var(--space-2);
			min-width: 0;
			:global(.r-icon) {
				color: var(--text-secondary);
				flex: none;
			}
			:global(.r-icon.folder) {
				color: var(--primary);
			}
			.nm {
				overflow: hidden;
				text-overflow: ellipsis;
				white-space: nowrap;
				color: var(--text-primary);
				font-weight: var(--fw-medium);
				font-size: var(--fs-sm);
			}
		}
		.r-size,
		.r-date {
			font-size: var(--fs-xs);
			color: var(--text-muted);
			font-family: var(--font-mono);
		}
		.r-actions {
			display: flex;
			gap: var(--space-1);
			justify-content: flex-end;
		}
		.r-act {
			background: transparent;
			border: none;
			color: var(--text-muted);
			cursor: pointer;
			display: flex;
			padding: var(--space-1);
			border-radius: var(--radius-sm);
			opacity: 0;
			transition: opacity var(--dur) var(--ease), color var(--dur) var(--ease), background var(--dur) var(--ease);
			&:hover {
				color: var(--text-primary);
				background: var(--tint-soft);
			}
			&.danger:hover {
				color: var(--danger);
				background: var(--danger-soft);
			}
		}
	}

	@media (max-width: 640px) {
		.resource-area.grid {
			grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
			gap: var(--space-3);
		}
		.search-box {
			order: -1;
			flex-basis: 100%;
		}
		.toolbar-right {
			width: 100%;
			justify-content: space-between;
		}
		.list-head,
		.row {
			grid-template-columns: 1fr 70px 80px;
		}
		.list-head span:nth-child(3),
		.row .r-date {
			display: none;
		}
		.bulk-bar .bulk-btn span {
			display: none;
		}
	}
</style>
