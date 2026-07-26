<script>
	import Icon from '$lib/ui/Icon.svelte';
	import ConfirmModal from '$lib/components/ConfirmModal.svelte';
	import { FrontendClient } from '$lib/frontendClient';
	import { onMount } from 'svelte';
	import { toast } from '$lib/toast.js';
	import { useQueryClient } from '@tanstack/svelte-query';
	import { glyphForMime } from '$lib/ui/icons.js';
	import { TRASH_TTL_DAYS, autoDeleteIn } from '$lib/retention.js';

	const queryClient = useQueryClient();

	// Restore/permanent-delete change quota usage; refresh the shared storage
	// query so the sidebar + dashboard meters update without a manual reload.
	function refreshStorage() {
		queryClient.invalidateQueries({ queryKey: ['fetchStorageStats'] });
	}

	let trashItems = $state([]);
	let loading = $state(true);
	let busy = $state(false);

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
			// Most recently deleted first: that is what someone opening the trash
			// is nearly always looking for.
			trashItems = [...folders, ...files].sort(
				(a, b) => new Date(b.deleted_on || 0) - new Date(a.deleted_on || 0)
			);
		} catch (e) {
			console.error(e);
			toast.error('Could not load trash', 'Check your connection and try again.');
		} finally {
			loading = false;
		}
	}

	let isEmpty = $derived(!loading && trashItems.length === 0);

	let rows = $derived(
		trashItems.map((item) => ({
			...item,
			glyph: item.type === 'folder' ? 'folder' : glyphForMime(item.mime, item.name),
			sizeLabel: item.type === 'folder' ? `${item.count ?? 0} items` : formatSize(item.size),
			deletedLabel: formatDate(item.deleted_on),
			due: autoDeleteIn(item.deleted_on)
		}))
	);

	function formatSize(bytes) {
		if (!bytes) return '0 B';
		const k = 1024;
		const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
		const i = Math.min(Math.floor(Math.log(bytes) / Math.log(k)), sizes.length - 1);
		return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
	}

	function formatDate(value) {
		if (!value) return '-';
		return new Date(value).toLocaleDateString(undefined, { day: 'numeric', month: 'short' });
	}

	// ---- actions -----------------------------------------------------------
	async function handleRestore(item) {
		try {
			const endpoint =
				item.type === 'folder' ? '/api/v1/sanctum/folder/restore' : '/api/v1/sanctum/file/restore';
			const payload = item.type === 'folder' ? { folder_id: item.id } : { file_id: item.id };
			await FrontendClient.post(endpoint, payload);
			toast.success('Restored', item.name);
			refreshStorage();
			await loadTrash();
		} catch (e) {
			console.error(e);
			toast.error('Could not restore that item');
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
		} catch {
			deletedItemCount = 'unknown';
		}
	}

	async function confirmDelete() {
		if (!itemToDelete) return;
		busy = true;
		try {
			await permanentDelete(itemToDelete);
			toast.success('Deleted forever', itemToDelete.name);
			refreshStorage();
			await loadTrash();
		} catch (e) {
			console.error(e);
			toast.error('Could not delete that item');
		} finally {
			busy = false;
			showDeleteModal = false;
			itemToDelete = null;
		}
	}

	let showEmptyModal = $state(false);

	function handleEmptyTrash() {
		if (trashItems.length === 0) return;
		showEmptyModal = true;
	}

	async function confirmEmptyTrash() {
		busy = true;
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
		busy = false;
		showEmptyModal = false;
		toast.success('Trash emptied', `${ok} item${ok === 1 ? '' : 's'} permanently deleted.`);
		refreshStorage();
		await loadTrash();
	}

	let deleteMessage = $derived(
		itemToDelete?.type === 'folder'
			? deletedItemCount === null
				? `Permanently delete "${itemToDelete?.name}"? (counting what's inside…)`
				: deletedItemCount === 'unknown'
					? `Permanently delete "${itemToDelete?.name}"? This cannot be undone.`
					: `Permanently delete "${itemToDelete?.name}" and the ${deletedItemCount} items inside? This cannot be undone.`
			: `Permanently delete "${itemToDelete?.name}"? This cannot be undone.`
	);
</script>

<div class="trash">
	<header class="head">
		<div class="head-text">
			<h1>Trash</h1>
			<span class="sub">Items stay here for {TRASH_TTL_DAYS} days, then delete themselves.</span>
		</div>
		<button
			type="button"
			class="empty-btn"
			disabled={trashItems.length === 0 || busy}
			onclick={handleEmptyTrash}
		>
			<Icon name="trash" size={15} />
			Empty Trash
		</button>
	</header>

	{#if loading}
		<div class="table">
			{#each Array(5) as _, i (i)}
				<div class="row">
					<span class="sk" style="width:{160 + i * 40}px"></span>
				</div>
			{/each}
		</div>
	{:else if isEmpty}
		<div class="empty">
			<Icon name="trash" size={34} stroke={1.2} />
			<span class="empty-title">Trash is empty</span>
			<span class="empty-line">
				Deleted items land here and self-destruct after {TRASH_TTL_DAYS} days.
			</span>
		</div>
	{:else}
		<div class="table">
			<div class="thead">
				<span class="c-name">Name</span>
				<span class="c-size">Size</span>
				<span class="c-deleted">Deleted</span>
				<span class="c-due">Auto-delete</span>
				<span class="c-actions"></span>
			</div>

			{#each rows as r (r.type + r.id)}
				<div class="row">
					<div class="c-name">
						<span class="glyph"><Icon name={r.glyph} size={16} /></span>
						<span class="name" title={r.name}>{r.name}</span>
					</div>
					<span class="c-size mono">{r.sizeLabel}</span>
					<span class="c-deleted mono">{r.deletedLabel}</span>
					<span class="c-due mono" class:soon={r.due.soon}>{r.due.label}</span>
					<div class="c-actions">
						<button
							type="button"
							class="act"
							title="Restore"
							aria-label="Restore {r.name}"
							onclick={() => handleRestore(r)}
						>
							<Icon name="restore" size={15} />
						</button>
						<button
							type="button"
							class="act danger"
							title="Delete forever"
							aria-label="Delete {r.name} forever"
							onclick={() => handleDelete(r)}
						>
							<Icon name="trash" size={15} />
						</button>
					</div>
				</div>
			{/each}
		</div>
	{/if}
</div>

<ConfirmModal
	bind:show={showDeleteModal}
	title="Delete forever?"
	message={deleteMessage}
	confirmLabel="Delete forever"
	icon="trash"
	isDanger
	{busy}
	onconfirm={confirmDelete}
/>

<ConfirmModal
	bind:show={showEmptyModal}
	title="Empty the trash?"
	message={`This permanently deletes all ${trashItems.length} item${trashItems.length === 1 ? '' : 's'}. Files cannot be recovered afterwards.`}
	confirmLabel="Empty trash"
	icon="trash"
	isDanger
	{busy}
	onconfirm={confirmEmptyTrash}
/>

<style lang="scss">
	.trash {
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
		min-height: 100%;
	}

	.head {
		display: flex;
		align-items: flex-end;
		justify-content: space-between;
		gap: var(--space-4);
		padding: var(--space-2) 0.125rem 0;
	}

	.head-text {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);

		h1 {
			margin: 0;
			font-size: var(--fs-h2);
			font-weight: var(--fw-black);
			letter-spacing: var(--tracking-tight);
			line-height: var(--lh-tight);
		}
	}

	.sub {
		font-size: var(--fs-sm);
		color: var(--ink-faint);
	}

	.empty-btn {
		display: flex;
		align-items: center;
		gap: 0.4375rem;
		height: 34px;
		padding-inline: 0.875rem;
		border-radius: var(--radius-md);
		border: 1px solid var(--edge);
		background: none;
		font: inherit;
		font-size: var(--fs-sm);
		font-weight: var(--fw-medium);
		color: var(--danger);
		cursor: pointer;
		flex: 0 0 auto;
		transition:
			background var(--dur-fast) var(--ease),
			color var(--dur-fast) var(--ease);

		&:hover:not(:disabled) {
			background: var(--danger-soft);
		}
		&:disabled {
			color: var(--ink-faint);
			cursor: not-allowed;
		}
	}

	.table {
		border: 1px solid var(--edge);
		border-radius: var(--radius-md);
		background: var(--surface);
		overflow: hidden;
	}

	.thead,
	.row {
		display: flex;
		align-items: center;
		gap: var(--space-4);
		padding: 0.5625rem 1rem;
		border-bottom: 1px solid var(--edge);
	}

	.thead {
		padding-block: 0.625rem;
		font-size: var(--fs-xs);
		color: var(--ink-faint);
	}

	.row {
		transition: background var(--dur-fast) var(--ease);

		&:last-child {
			border-bottom: 0;
		}
		&:hover {
			background: var(--surface-hover);
		}
	}

	.c-name {
		flex: 1;
		min-width: 0;
		display: flex;
		align-items: center;
		gap: 0.625rem;
	}

	.glyph {
		flex: 0 0 auto;
		display: grid;
		place-items: center;
		color: var(--ink-mute);
	}

	.name {
		font-size: var(--fs-sm);
		font-weight: var(--fw-medium);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.c-size {
		flex: 0 0 90px;
		text-align: right;
	}
	.c-deleted {
		flex: 0 0 110px;
		text-align: right;
	}
	.c-due {
		flex: 0 0 96px;
		text-align: right;
		color: var(--ink-faint);

		/* The last few days get the one warning colour this screen uses. */
		&.soon {
			color: var(--warn);
		}
	}
	.c-actions {
		flex: 0 0 64px;
		display: flex;
		justify-content: flex-end;
		gap: 0.125rem;
	}

	.mono {
		font-family: var(--font-mono);
		font-size: var(--fs-xs);
		color: var(--ink-faint);
	}

	.act {
		width: 28px;
		height: 28px;
		border: 0;
		background: none;
		border-radius: var(--radius-sm);
		display: grid;
		place-items: center;
		color: var(--ink-faint);
		cursor: pointer;
		transition:
			background var(--dur-fast) var(--ease),
			color var(--dur-fast) var(--ease);

		&:hover {
			background: var(--tint-softer);
			color: var(--ink);
		}
		&.danger {
			color: var(--danger);

			&:hover {
				background: var(--danger-soft);
			}
		}
	}

	.empty {
		flex: 1;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 0.875rem;
		border: 1px solid var(--edge);
		border-radius: var(--radius-md);
		background: var(--surface);
		padding: 4rem 1rem;
		text-align: center;
		color: var(--ink-faint);
	}

	.empty-title {
		font-size: var(--fs-lg);
		font-weight: var(--fw-medium);
		letter-spacing: var(--tracking-tight);
		color: var(--ink);
	}

	.empty-line {
		font-size: var(--fs-sm);
		color: var(--ink-mute);
	}

	.sk {
		display: block;
		height: 0.9rem;
		border-radius: var(--radius-sm);
		background: var(--tint-softer);
	}

	@media (max-width: 760px) {
		.thead {
			display: none;
		}
		.row {
			flex-wrap: wrap;
		}
		.c-size,
		.c-deleted {
			flex: 0 0 auto;
		}
		.c-due {
			flex: 1;
			text-align: left;
		}
	}
</style>
