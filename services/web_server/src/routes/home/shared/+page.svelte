<script>
	import FolderCard from '$lib/components/FolderCard.svelte';
	import FileCard from '$lib/components/FileCard.svelte';
	import ShareModal from '$lib/components/ShareModal.svelte';
	import ResourceToolbar from '$lib/components/ResourceToolbar.svelte';
	import Icon from '@iconify/svelte';
	import { flip } from 'svelte/animate';
	import { FrontendClient } from '$lib/frontendClient.js';
	import { browser } from '$app/environment';
	import { createQuery, useQueryClient } from '@tanstack/svelte-query';
	import { toast } from 'svelte-sonner';
	import axios from 'axios';
	import { downloadFile } from '$lib/download.js';
	import { EmptyState, Prompt, Skeleton } from '$lib/ui';

	let passwordPrompt = $state({ open: false, file: null });
	function handleDownload(file) {
		if (file.encrypted) passwordPrompt = { open: true, file };
		else downloadFile(file);
	}
	function submitPassword(pw) {
		const f = passwordPrompt.file;
		passwordPrompt = { open: false, file: null };
		if (f && pw) downloadFile(f, { password: pw });
	}

	async function fetchSharedFilesFn() {
		try {
			let { data } = await FrontendClient.get('/api/v1/sanctum/file/list', { params: { shared: true } });
			if (data?.status === 200) return data?.data?.files || [];
			throw new Error(data.message || 'Unknown error');
		} catch (e) {
			console.error('Error fetching shared files:', e);
			return [];
		}
	}
	const fetchSharedFiles = createQuery(() => ({ queryKey: ['fetchSharedFiles'], queryFn: fetchSharedFilesFn, enabled: browser }));

	async function fetchSharedFoldersFn() {
		try {
			let { data } = await FrontendClient.post('/api/v1/sanctum/folder/list', { shared: true });
			return data?.data?.folders || [];
		} catch (e) {
			console.error('Error fetching shared folders:', e);
			return [];
		}
	}
	const fetchSharedFolders = createQuery(() => ({ queryKey: ['fetchSharedFolders'], queryFn: fetchSharedFoldersFn, enabled: browser }));

	const queryClient = useQueryClient();

	let loading = $derived(fetchSharedFiles.isLoading || fetchSharedFolders.isLoading);
	let rawFolders = $derived(fetchSharedFolders?.data || []);
	let rawFiles = $derived(fetchSharedFiles?.data || []);

	// ---- search / sort / view ----
	let search = $state('');
	let sortKey = $state('name');
	let sortDir = $state('asc');
	let view = $state('grid');

	function match(name) {
		const q = search.trim().toLowerCase();
		return !q || (name || '').toLowerCase().includes(q);
	}
	function sortList(list, isFolder) {
		const dir = sortDir === 'asc' ? 1 : -1;
		return [...list].sort((a, b) => {
			if (sortKey === 'size') {
				const sa = isFolder ? 0 : Number(a.size) || 0;
				const sb = isFolder ? 0 : Number(b.size) || 0;
				if (sa !== sb) return dir * (sa - sb);
			} else if (sortKey === 'date') {
				const da = new Date(a.created_on || 0).getTime();
				const db = new Date(b.created_on || 0).getTime();
				if (da !== db) return dir * (da - db);
			}
			return dir * (a.name || '').localeCompare(b.name || '');
		});
	}
	let folders = $derived(sortList(rawFolders.filter((f) => match(f.name)), true));
	let files = $derived(sortList(rawFiles.filter((f) => match(f.name)), false));
	let isEmpty = $derived(folders.length === 0 && files.length === 0);

	// ---- share modal ----
	let showShareModal = $state(false);
	let itemToShare = $state(null);
	function handleShare(item) {
		itemToShare = item;
		showShareModal = true;
	}

	function formatSize(bytes) {
		if (!bytes) return '0 B';
		const k = 1024;
		const s = ['B', 'KB', 'MB', 'GB', 'TB'];
		const i = Math.min(Math.floor(Math.log(bytes) / Math.log(k)), s.length - 1);
		return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + s[i];
	}
	function relTime(dateString) {
		const then = new Date(dateString).getTime();
		if (!then) return '';
		const s = Math.max(0, (Date.now() - then) / 1000);
		if (s < 60) return 'just now';
		if (s < 3600) return `${Math.floor(s / 60)}m ago`;
		if (s < 86400) return `${Math.floor(s / 3600)}h ago`;
		if (s < 86400 * 7) return `${Math.floor(s / 86400)}d ago`;
		return new Date(dateString).toLocaleDateString(undefined, { month: 'short', day: 'numeric' });
	}
	function getType(mime) {
		if (mime?.includes('image')) return 'image';
		if (mime?.includes('video')) return 'video';
		if (mime?.includes('audio')) return 'audio';
		if (mime?.includes('pdf') || mime?.includes('document')) return 'doc';
		return 'file';
	}
	const typeIcons = {
		image: 'ri:image-line', video: 'ri:film-line', audio: 'ri:music-2-line',
		doc: 'ri:file-text-line', file: 'ri:file-3-line'
	};

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
			queryClient.invalidateQueries({ queryKey: ['fetchFiles'] });
		} catch (e) {
			toast.error('Failed to update star status');
		}
	}
</script>

<div class="view">
	<header class="page-head">
		<div>
			<h1 class="page-title">Shared</h1>
			<p class="page-subtitle">Files and folders you're sharing with others.</p>
		</div>
	</header>

	<ResourceToolbar bind:search bind:sortKey bind:sortDir bind:view placeholder="Search shared" />

	{#if loading}
		<div class="grid">
			{#each Array(4) as _, i (i)}<Skeleton height="118px" radius="var(--radius-md)" />{/each}
		</div>
	{:else if isEmpty}
		<EmptyState icon="ri:share-line" title="Nothing shared yet" line={search ? 'No shared items match your search.' : 'Share a file or folder and it shows up here.'} />
	{:else if view === 'grid'}
		<div class="grid">
			{#each folders as folder (folder.id)}
				<div animate:flip={{ duration: 200 }}>
					<FolderCard name={folder.name} count={folder.count} starred={folder.starred}
						onshare={() => handleShare(folder)} onstar={() => handleStar(folder, 'folder')} />
				</div>
			{/each}
			{#each files as file (file.id)}
				<div animate:flip={{ duration: 200 }}>
					<FileCard name={file.name} size={formatSize(file.size)} date={relTime(file.created_on)}
						type={getType(file.mime)} encrypted={file.encrypted} starred={file.starred}
						ondownload={() => handleDownload(file)} onshare={() => handleShare(file)}
						onstar={() => handleStar(file, 'file')} />
				</div>
			{/each}
		</div>
	{:else}
		<div class="list">
			{#each folders as folder (folder.id)}
				<div class="row">
					<span class="ic"><Icon icon="ri:folder-3-line" width="16" /></span>
					<span class="name">{folder.name}</span>
					<span class="meta">{folder.count ?? 0} items</span>
					<span class="meta" />
					<div class="row-actions">
						<button aria-label="Share" onclick={() => handleShare(folder)}><Icon icon="ri:share-forward-line" width="16" /></button>
						<button aria-label="Star" onclick={() => handleStar(folder, 'folder')}><Icon icon={folder.starred ? 'ri:star-fill' : 'ri:star-line'} width="16" /></button>
					</div>
				</div>
			{/each}
			{#each files as file (file.id)}
				<div class="row">
					<span class="ic"><Icon icon={typeIcons[getType(file.mime)]} width="16" /></span>
					<span class="name">{file.name}{#if file.encrypted}<Icon icon="ri:lock-2-line" width="12" class="lk" />{/if}</span>
					<span class="meta">{formatSize(file.size)}</span>
					<span class="meta">{relTime(file.created_on)}</span>
					<div class="row-actions">
						<button aria-label="Download" onclick={() => handleDownload(file)}><Icon icon="ri:download-line" width="16" /></button>
						<button aria-label="Share" onclick={() => handleShare(file)}><Icon icon="ri:share-forward-line" width="16" /></button>
						<button aria-label="Star" onclick={() => handleStar(file, 'file')}><Icon icon={file.starred ? 'ri:star-fill' : 'ri:star-line'} width="16" /></button>
					</div>
				</div>
			{/each}
		</div>
	{/if}
</div>

<Prompt
	open={passwordPrompt.open}
	title={passwordPrompt.file ? `Decrypt ${passwordPrompt.file.name}` : 'Decrypt file'}
	message="This file is encrypted. Enter the password to download and decrypt it."
	placeholder="Password"
	type="password"
	submitLabel="Download"
	onsubmit={submitPassword}
	onclose={() => (passwordPrompt = { open: false, file: null })}
/>

{#if showShareModal}
	<ShareModal
		item={itemToShare}
		on:close={() => {
			showShareModal = false;
			queryClient.invalidateQueries({ queryKey: ['fetchSharedFiles'] });
			queryClient.invalidateQueries({ queryKey: ['fetchSharedFolders'] });
		}}
	/>
{/if}

<style lang="scss">
	.grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
		grid-auto-rows: 1fr;
		gap: var(--space-4);
	}

	.list {
		display: flex;
		flex-direction: column;
		border: 1px solid var(--edge);
		border-radius: var(--radius-md);
		overflow: hidden;
	}
	.row {
		display: grid;
		grid-template-columns: 32px 1fr 90px 90px auto;
		align-items: center;
		gap: var(--space-3);
		padding: var(--space-2) var(--space-4);
		background: var(--surface);
		transition: background var(--dur-fast) var(--ease);

		& + .row {
			border-top: 1px solid var(--edge);
		}
		&:hover {
			background: var(--surface-hover);
		}
		&:hover .row-actions {
			opacity: 1;
		}
	}
	.ic {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 32px;
		height: 32px;
		border: 1px solid var(--edge);
		border-radius: var(--radius-sm);
		color: var(--ink-faint);
	}
	.name {
		display: inline-flex;
		align-items: center;
		gap: var(--space-2);
		font-size: var(--fs-sm);
		font-weight: var(--fw-medium);
		color: var(--ink);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		:global(.lk) {
			color: var(--ink-faint);
			flex-shrink: 0;
		}
	}
	.meta {
		font-family: var(--font-mono);
		font-size: var(--fs-xs);
		color: var(--ink-faint);
	}
	.row-actions {
		display: flex;
		gap: 2px;
		opacity: 0;
		transition: opacity var(--dur) var(--ease);

		button {
			display: flex;
			align-items: center;
			justify-content: center;
			width: 28px;
			height: 28px;
			background: none;
			border: none;
			border-radius: var(--radius-sm);
			color: var(--ink-mute);
			cursor: pointer;
			&:hover {
				background: var(--tint-soft);
				color: var(--ink);
			}
		}
	}

	@media (max-width: 620px) {
		.grid {
			grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
			gap: var(--space-3);
		}
		.row {
			grid-template-columns: 32px 1fr auto;
		}
		.row .meta {
			display: none;
		}
		.row-actions {
			opacity: 1;
		}
	}
</style>
