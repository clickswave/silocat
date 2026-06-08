<script>
	import Icon from '@iconify/svelte';
	import FileCard from '$lib/components/FileCard.svelte';
	import FolderRow from '$lib/components/FolderCard.svelte';
	import ConfirmModal from '$lib/components/ConfirmModal.svelte';
	// Let's keep alias FolderRow -> FolderCard since I used FolderRow in template.

	import { FrontendClient } from '$lib/frontendClient';
	import { onMount } from 'svelte';
	import { toast } from 'svelte-sonner';

	let trashItems = [];
	let loading = true;

	onMount(async () => {
		await loadTrash();
	});

	async function loadTrash() {
		loading = true;
		try {
			// Fetch deleted folders
			const folderRes = await FrontendClient.post('/api/v1/sanctum/folder/list', {
				deleted: true
			});

			// Fetch deleted files
			const fileRes = await FrontendClient.post('/api/v1/sanctum/file/list', {
				deleted: true
			});

			let files = [];
			let folders = [];

			if (folderRes.data.status === 200) {
				folders = folderRes.data.data.folders.map((f) => ({ ...f, type: 'folder' }));
			}
			if (fileRes.data.status === 200) {
				files = fileRes.data.data.files.map((f) => ({ ...f, type: 'file' }));
			}

			// Combine and sort by date? Or just list folders then files?
			// Let's combine.
			trashItems = [...folders, ...files].sort((a, b) => {
				// assume created_on is what we have, technically we might want deleted_on but schema didn't seem to have it?
				// Checking SQL... we just used `deleted=true` and `created_on`.
				// Wait, if no deleted_on timestamp, we can't show "Deleted X days ago" accurately unless we track it.
				// Schema check: "created_on" is available. "modified_on"?
				return new Date(b.created_on) - new Date(a.created_on);
			});
		} catch (e) {
			console.error(e);
			toast.error('Failed to load trash');
		} finally {
			loading = false;
		}
	}

	function formatSize(bytes) {
		if (bytes === 0) return '0 B';
		const k = 1024;
		const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
		const i = Math.floor(Math.log(bytes) / Math.log(k));
		return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
	}

	function formatTime(dateString) {
		const date = new Date(dateString);
		return date.toLocaleDateString() + ' ' + date.toLocaleTimeString();
	}

	function getFileType(mime) {
		if (!mime) return 'file';
		if (mime.includes('image')) return 'image';
		if (mime.includes('video')) return 'video';
		if (mime.includes('audio')) return 'audio';
		if (mime.includes('pdf') || mime.includes('document')) return 'doc';
		return 'file';
	}

	/* Actions */
	async function handleRestore(item) {
		loading = true;
		try {
			const endpoint =
				item.type === 'folder' ? '/api/v1/sanctum/folder/restore' : '/api/v1/sanctum/file/restore';

			const payload = item.type === 'folder' ? { folder_id: item.id } : { file_id: item.id };

			await FrontendClient.post(endpoint, payload);
			toast.success('Restored ' + item.name);
			await loadTrash(); // Refresh list
		} catch (e) {
			console.error(e);
			toast.error('Failed to restore item');
			loading = false;
		}
	}

	let showDeleteModal = false;
	let itemToDelete = null;
	let deletedItemCount = null;

	function handleDelete(item) {
		itemToDelete = item;
		deletedItemCount = null;

		if (item.type === 'folder') {
			fetchDeletionStats(item.id);
		}

		showDeleteModal = true;
	}

	async function fetchDeletionStats(folderId) {
		try {
			const res = await FrontendClient.post('/api/v1/sanctum/folder/stats', {
				folder_id: folderId
			});
			if (res.data?.data) {
				deletedItemCount = res.data.data.total_items;
			}
		} catch (e) {
			console.error('Failed to fetch folder stats', e);
			deletedItemCount = 'unknown';
		}
	}

	async function confirmDelete() {
		showDeleteModal = false;
		if (!itemToDelete) return;

		loading = true;
		try {
			const endpoint =
				itemToDelete.type === 'folder'
					? '/api/v1/sanctum/folder/permanent-delete'
					: '/api/v1/sanctum/file/permanent-delete';

			const payload =
				itemToDelete.type === 'folder'
					? { folder_id: itemToDelete.id }
					: { file_id: itemToDelete.id };

			await FrontendClient.post(endpoint, payload);
			toast.success('Deleted ' + itemToDelete.name);
			await loadTrash(); // Refresh list
		} catch (e) {
			console.error(e);
			toast.error('Failed to delete item');
			loading = false;
		}
	}

	function handleEmptyTrash() {
		toast('Empty Trash not yet implemented');
	}
</script>

<div class="page-container">
	<header class="page-header">
		<div class="title-group">
			<h1>Trash</h1>
			<p>Items are kept here until you permanently delete them.</p>
		</div>
		<button class="btn btn-ghost" on:click={handleEmptyTrash}>Empty Trash</button>
	</header>

	{#if loading}
		<div class="loading-state">
			<Icon icon="ri:loader-4-line" class="spinner" />
			<span>Loading trash...</span>
		</div>
	{:else if trashItems.length === 0}
		<div class="empty-state">
			<Icon icon="ri:delete-bin-line" width="48" style="opacity: 0.3" />
			<p>Trash is empty</p>
		</div>
	{:else}
		<div class="grid-layout">
			{#each trashItems as item}
				{#if item.type === 'folder'}
					<FolderRow
						name={item.name}
						count={item.count || 0}
						starred={item.starred}
						isTrash={true}
						onrestore={() => handleRestore(item)}
						ondelete={() => handleDelete(item)}
						onclick={() => toast('Restore folder to view contents')}
					/>
				{:else}
					<FileCard
						name={item.name}
						size={formatSize(item.size)}
						type={getFileType(item.mime)}
						starred={item.starred}
						isTrash={true}
						onrestore={() => handleRestore(item)}
						ondelete={() => handleDelete(item)}
					/>
				{/if}
			{/each}
		</div>
	{/if}

	<ConfirmModal
		bind:show={showDeleteModal}
		title="Permanently Delete?"
		message={`Are you sure you want to permanently delete "${itemToDelete?.name}"? ${
			itemToDelete?.type === 'folder'
				? deletedItemCount !== null
					? deletedItemCount === 'unknown'
						? ''
						: `This includes ${deletedItemCount} items inside.`
					: '(Calculating items...)'
				: 'This action cannot be undone.'
		}`}
		confirmLabel="Permanently Delete"
		isDanger={true}
		onconfirm={confirmDelete}
	/>
</div>

<style lang="scss">
	.page-container {
		width: 100%;
		height: 100%;
		display: flex;
		flex-direction: column;
		gap: var(--space-6);
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
	}

	.grid-layout {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
		gap: var(--space-5);
		padding-bottom: var(--space-6);
	}

	.loading-state,
	.empty-state {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		padding: var(--space-10);
		color: var(--text-secondary);
		gap: var(--space-4);

		.spinner {
			animation: spin 1s linear infinite;
		}
	}

	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
	}

	@media (max-width: 600px) {
		.grid-layout {
			grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
			gap: var(--space-3);
		}
	}
</style>
