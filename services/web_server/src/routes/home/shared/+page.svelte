<script>
	import FolderCard from '$lib/components/FolderCard.svelte';
	import FileCard from '$lib/components/FileCard.svelte';
	import ShareModal from '$lib/components/ShareModal.svelte';
	import { flip } from 'svelte/animate';
	import { FrontendClient } from '$lib/frontendClient.js';
	import { browser } from '$app/environment';
	import { createQuery, useQueryClient } from '@tanstack/svelte-query';
	import Icon from '@iconify/svelte';
	import { toast } from 'svelte-sonner';
	import axios from 'axios';
	import { fade } from 'svelte/transition';
	import { downloadFile } from '$lib/download.js';

	function handleDownload(file) {
		if (file.encrypted) {
			const pw = window.prompt(`"${file.name}" is encrypted. Enter the password to decrypt it:`);
			if (!pw) return;
			downloadFile(file, { password: pw });
		} else {
			downloadFile(file);
		}
	}

	// Fetch Shared Files
	async function fetchSharedFilesFn() {
		try {
			// Using GET proxy with shared param
			let { data } = await FrontendClient.get('/api/v1/sanctum/file/list', {
				params: { shared: true }
			});

			if (data?.status === 200) {
				return data?.data?.files || [];
			} else {
				throw new Error(data.message || 'Unknown error');
			}
		} catch (e) {
			console.error('Error fetching shared files:', e);
			return [];
		}
	}

	const fetchSharedFiles = createQuery(() => ({
		queryKey: ['fetchSharedFiles'],
		queryFn: fetchSharedFilesFn,
		enabled: browser
	}));

	// Fetch Shared Folders
	async function fetchSharedFoldersFn() {
		try {
			// Using POST proxy with shared body
			let { data } = await FrontendClient.post('/api/v1/sanctum/folder/list', {
				shared: true
			});

			if (data && data.data && data.data.folders) {
				return data.data.folders;
			}
			return [];
		} catch (e) {
			console.error('Error fetching shared folders:', e);
			return [];
		}
	}

	const fetchSharedFolders = createQuery(() => ({
		queryKey: ['fetchSharedFolders'],
		queryFn: fetchSharedFoldersFn,
		enabled: browser
	}));

	const queryClient = useQueryClient();

	let folders = $derived(fetchSharedFolders?.data || []);
	let files = $derived(fetchSharedFiles?.data || []);

	// State for Share Modal
	let showShareModal = $state(false);
	let itemToShare = $state(null);

	function handleShare(item) {
		itemToShare = item;
		showShareModal = true;
	}

	// Helpers
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

	// Reuse Star logic just in case user wants to star from here
	async function handleStar(item, type) {
		const newStatus = !item.starred;
		try {
			await axios.post(`/api/v1/sanctum/${type}/star`, {
				[type === 'file' ? 'file_id' : 'folder_id']: item.id,
				starred: newStatus
			});
			toast.success(newStatus ? 'Added to starred' : 'Removed from starred');
			queryClient.invalidateQueries({ queryKey: ['fetchSharedFiles'] });
			queryClient.invalidateQueries({ queryKey: ['fetchSharedFolders'] });
			// Invalidate others just in case
			queryClient.invalidateQueries({ queryKey: ['fetchFiles'] });
		} catch (e) {
			console.error('Star failed:', e);
			toast.error('Failed to update star status');
		}
	}
</script>

<div class="page-container">
	<header class="page-header">
		<div class="title-group">
			<h1>Shared</h1>
			<p>Files and folders you are sharing with others</p>
		</div>
	</header>

	<div class="resource-grid">
		<!-- Folders -->
		{#each folders as folder (folder.id)}
			<div animate:flip={{ duration: 300 }}>
				<FolderCard
					name={folder.name}
					count={folder.count}
					starred={folder.starred}
					onshare={() => handleShare(folder)}
					onstar={() => handleStar(folder, 'folder')}
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
					starred={file.starred}
					ondownload={() => handleDownload(file)}
					onshare={() => handleShare(file)}
					onstar={() => handleStar(file, 'file')}
				/>
			</div>
		{/each}

		{#if folders.length === 0 && files.length === 0}
			<div class="empty-state">
				<Icon icon="ri:share-line" width="64" />
				<p>No shared items</p>
				<div class="sub-text">Share files or folders to manage them here</div>
			</div>
		{/if}
	</div>
</div>

{#if showShareModal}
	<ShareModal
		item={itemToShare}
		on:close={() => {
			showShareModal = false;
			// Refresh logic if needed when closing modal (e.g. if they turned off sharing)
			queryClient.invalidateQueries({ queryKey: ['fetchSharedFiles'] });
			queryClient.invalidateQueries({ queryKey: ['fetchSharedFolders'] });
		}}
	/>
{/if}

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

	.resource-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
		gap: var(--space-5);
		padding-bottom: var(--space-8);

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
				font-size: var(--fs-lg);
				font-weight: var(--fw-medium);
				margin: 0;
			}
			.sub-text {
				font-size: var(--fs-sm);
				color: var(--text-muted);
			}
		}
	}

	@media (max-width: 600px) {
		.resource-grid {
			grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
			gap: var(--space-3);
		}
	}
</style>
