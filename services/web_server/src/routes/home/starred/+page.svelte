<script>
	import FolderCard from '$lib/components/FolderCard.svelte';
	import FileCard from '$lib/components/FileCard.svelte';
	import { flip } from 'svelte/animate';
	import { FrontendClient } from '$lib/frontendClient.js';
	import { browser } from '$app/environment';
	import { createQuery, useQueryClient } from '@tanstack/svelte-query';
	import Icon from '@iconify/svelte';
	import { toast } from 'svelte-sonner';
	import axios from 'axios';
	import { fade } from 'svelte/transition';

	// Fetch Starred Files
	async function fetchStarredFilesFn() {
		try {
			// Using GET proxy with starred param
			let { data } = await FrontendClient.get('/api/v1/sanctum/file/list', {
				params: { starred: true }
			});

			if (data?.status === 200) {
				return data?.data?.files || [];
			} else {
				throw new Error(data.message || 'Unknown error');
			}
		} catch (e) {
			console.error('Error fetching starred files:', e);
			return [];
		}
	}

	const fetchStarredFiles = createQuery(() => ({
		queryKey: ['fetchStarredFiles'],
		queryFn: fetchStarredFilesFn,
		enabled: browser
	}));

	// Fetch Starred Folders
	async function fetchStarredFoldersFn() {
		try {
			// Using POST proxy with starred body
			let { data } = await FrontendClient.post('/api/v1/sanctum/folder/list', {
				starred: true
			});

			if (data && data.data && data.data.folders) {
				return data.data.folders;
			}
			return [];
		} catch (e) {
			console.error('Error fetching starred folders:', e);
			return [];
		}
	}

	const fetchStarredFolders = createQuery(() => ({
		queryKey: ['fetchStarredFolders'],
		queryFn: fetchStarredFoldersFn,
		enabled: browser
	}));

	const queryClient = useQueryClient();

	let folders = $derived(
		fetchStarredFolders?.data?.map((f) => ({
			...f,
			starred: true // Implicitly true since we fetched starred list, but backend returns it too
		})) || []
	);

	let files = $derived(
		fetchStarredFiles?.data?.map((file) => ({
			...file,
			starred: true
		})) || []
	);

	// Handlers
	async function handleStar(item, type) {
		// Toggle star (remove from list in this view)
		const newStatus = !item.starred;
		try {
			await axios.post(`/api/v1/sanctum/${type}/star`, {
				[type === 'file' ? 'file_id' : 'folder_id']: item.id,
				starred: newStatus
			});
			toast.success(newStatus ? 'Added to starred' : 'Removed from starred');
			queryClient.invalidateQueries({ queryKey: ['fetchStarredFiles'] });
			queryClient.invalidateQueries({ queryKey: ['fetchStarredFolders'] });
			// Also invalidate global lists just in case
			queryClient.invalidateQueries({ queryKey: ['fetchFiles'] });
			queryClient.invalidateQueries({ queryKey: ['fetchFolders'] });
		} catch (e) {
			console.error('Star failed:', e);
			toast.error('Failed to update star status');
		}
	}

	// Helpers (copied/simplified)
	function formatSize(bytes) {
		if (bytes === 0) return '0 B';
		const k = 1024;
		const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
		const i = Math.floor(Math.log(bytes) / Math.log(k));
		return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
	}

	function formatTime(dateString) {
		const date = new Date(dateString);
		const now = new Date();
		const diff = Math.floor((now - date) / 1000);
		if (diff < 60) return 'Just now';
		if (diff < 3600) return `${Math.floor(diff / 60)} min ago`;
		if (diff < 86400) return `${Math.floor(diff / 3600)} hour ago`;
		return `${Math.floor(diff / 86400)} days ago`;
	}

	function getFileType(mime) {
		if (mime.includes('image')) return 'image';
		if (mime.includes('video')) return 'video';
		if (mime.includes('audio')) return 'audio';
		if (mime.includes('pdf') || mime.includes('document')) return 'doc';
		return 'file';
	}

	function handleItemClick(e, item, type) {
		// Placeholder for selection or navigation logic if needed
		// For now, no navigation in Starred view? Or navigate to folder containing it?
		// Navigation might be complex since we don't have context of parent.
		// Let's just allow generic click.
	}
</script>

<div class="page-container">
	<header class="page-header">
		<div class="title-group">
			<h1>Starred</h1>
			<p>Your important files and folders</p>
		</div>
	</header>

	<div class="resource-grid">
		<!-- Folders -->
		{#each folders as folder (folder.id)}
			<div animate:flip={{ duration: 300 }}>
				<FolderCard
					name={folder.name}
					count={folder.count}
					starred={true}
					onstar={() => handleStar(folder, 'folder')}
					onclick={(e) => handleItemClick(e, folder, 'folder')}
				/>
			</div>
		{/each}

		<!-- Files -->
		{#each files as file (file.id)}
			<div animate:flip={{ duration: 300 }}>
				<FileCard
					name={file.name}
					size={formatSize(file.size)}
					date={formatTime(file.created_on)}
					type={getFileType(file.mime)}
					encrypted={file.encrypted}
					starred={true}
					onstar={() => handleStar(file, 'file')}
					onclick={(e) => handleItemClick(e, file, 'file')}
				/>
			</div>
		{/each}

		{#if folders.length === 0 && files.length === 0}
			<div class="empty-state">
				<Icon icon="ri:star-line" width="64" />
				<p>No starred items yet</p>
				<div class="sub-text">Star files or folders to access them quickly here</div>
			</div>
		{/if}
	</div>
</div>

<style lang="scss">
	.page-container {
		padding: 40px;
		max-width: 1600px;
		height: 100%;
		display: flex;
		flex-direction: column;
		gap: 32px;
		color: var(--text-primary);
		margin: 0 auto;
	}

	.page-header {
		.title-group {
			h1 {
				font-size: 24px;
				font-weight: 600;
				margin: 0 0 8px 0;
			}
			p {
				color: var(--text-muted);
				margin: 0;
				font-size: 16px;
			}
		}
	}

	.resource-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
		gap: 24px;
		padding-bottom: 40px;

		.empty-state {
			grid-column: 1 / -1;
			display: flex;
			flex-direction: column;
			align-items: center;
			justify-content: center;
			padding: 80px 0;
			gap: 16px;
			color: var(--text-muted);

			p {
				font-size: 18px;
				font-weight: 500;
				margin: 0;
			}
			.sub-text {
				font-size: 14px;
				opacity: 0.7;
			}
		}
	}
</style>
