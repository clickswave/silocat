<script>
	import StatsCard from '$lib/components/StatsCard.svelte';
	import FileCard from '$lib/components/FileCard.svelte';
	import FolderCard from '$lib/components/FolderCard.svelte';
	import Icon from '@iconify/svelte';
	import { FrontendClient } from '$lib/frontendClient.js';
	import { browser } from '$app/environment';
	import { createQuery } from '@tanstack/svelte-query';

	let { data } = $props();

	// --- Data Fetching ---
	async function fetchFilesFn() {
		try {
			const res = await FrontendClient.get('/api/v1/sanctum/file/list');
			return res.data?.success?.data?.files || [];
		} catch (e) {
			console.error('Error fetching recent files:', e);
			return [];
		}
	}

	const fetchFiles = createQuery(() => ({
		queryKey: ['fetchRecentFiles'],
		queryFn: fetchFilesFn,
		enabled: browser
	}));

	async function fetchFoldersFn() {
		try {
			const res = await FrontendClient.post('/api/v1/sanctum/folder/list', { parent_id: null });
			return res.data?.data?.folders || [];
		} catch (e) {
			console.error('Error fetching folders:', e);
			return [];
		}
	}

	const fetchFolders = createQuery(() => ({
		queryKey: ['fetchRootFolders'],
		queryFn: fetchFoldersFn,
		enabled: browser
	}));

	async function fetchStorageStatsFn() {
		try {
			let { data } = await FrontendClient.get('/api/v1/sanctum/user/storage');
			if (data?.success) {
				return data.success;
			}
			return { total: 0, used: 0, free: 0 };
		} catch (e) {
			console.error('Error fetching storage stats:', e);
			return { total: 0, used: 0, free: 0 };
		}
	}

	const fetchStorageStats = createQuery(() => ({
		queryKey: ['fetchStorageStats'],
		queryFn: fetchStorageStatsFn,
		enabled: browser
	}));

	// --- Derived Data ---
	let recentFiles = $derived(
		(fetchFiles?.data || [])
			.slice(0, 6) // Limit to 6 recent files
			.map((f) => ({
				...f,
				type: getFileType(f.mime)
			}))
	);

	let rootFolders = $derived(
		(fetchFolders?.data || [])
			.slice(0, 4) // Limit to 4 root folders
			.map((f) => ({
				...f,
				count: f.count || 0,
				color: 'blue'
			}))
	);

	function calculateStorage() {
		const fetched = fetchStorageStats?.data || { total: 0, used: 0, free: 0 };
		// Prefer the live stats total (base + all active subscriptions, incl. promos);
		// the session's totalAvailableSpace only carries one subscription and is stale.
		const total = fetched.total || data.user?.totalAvailableSpace;
		return {
			used: fetched.used,
			total: total,
			free: total - fetched.used
		};
	}

	let storage = $derived(calculateStorage());

	// Helpers
	function formatSize(bytes) {
		if (bytes === 0) return '0 B';
		const k = 1024;
		const sizes = ['B', 'KB', 'MB', 'GB'];
		const i = Math.min(Math.floor(Math.log(bytes) / Math.log(k)), sizes.length - 1);
		return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
	}

	function formatTime(dateString) {
		const date = new Date(dateString);
		return date.toLocaleDateString();
	}

	function getFileType(mime) {
		if (mime?.includes('image')) return 'image';
		if (mime?.includes('video')) return 'video';
		if (mime?.includes('audio')) return 'audio';
		if (mime?.includes('pdf') || mime?.includes('document')) return 'doc';
		return 'file';
	}
</script>

<div class="view dashboard">
	<header class="page-head">
		<div>
			<h1 class="page-title">Overview</h1>
			<p class="page-subtitle">
				Welcome back{data?.user?.username ? `, ${data.user.username}` : ''}.
			</p>
		</div>
		<a href="/home/files" class="btn btn-primary upload-cta">
			<Icon icon="ri:upload-cloud-2-line" width="18" /> Upload files
		</a>
	</header>

	<!-- Section 1: Subscription & Quota -->
	<div class="stats-section">
		<div class="card subscription-card">
			<div class="card-header">
				<Icon icon="ri:vip-crown-fill" width="24" color="var(--warning)" />
				<h3>Your Subscription</h3>
			</div>
			<div class="card-body">
				<div class="plan-info">
					<span class="plan-name">{data?.user?.subscription?.name || 'Free Plan'}</span>
					<span class="plan-status">Active</span>
				</div>
				<p class="plan-desc">
					{#if data?.user?.subscription}
						You have access to all premium features.
					{:else}
						Upgrade to Pro for more storage and features.
					{/if}
				</p>
				<button class="btn btn-ghost" onclick={() => (window.location.href = '/home/subscription')}
					>Manage Subscription</button
				>
			</div>
		</div>

		<div class="card quota-card">
			<div class="card-header">
				<Icon icon="ri:hard-drive-2-line" width="24" color="var(--primary)" />
				<h3>Storage Quota</h3>
			</div>
			<div class="card-body">
				<div class="usage-info">
					<span class="used">{formatSize(storage.used)} Used</span>
					<span class="total">of {formatSize(storage.total)}</span>
				</div>
				<div class="progress-bar">
					<div
						class="progress-fill"
						style="width: {storage.total ? (storage.used / storage.total) * 100 : 0}%"
					></div>
				</div>
				<p class="free-space">{formatSize(storage.free)} free</p>
			</div>
		</div>
	</div>

	<!-- Section 2: Recent Activity -->
	<div class="recent-section">
		<div class="section-header">
			<h2 class="section-title">Recent activity</h2>
			<a href="/home/files" class="view-all">View all files <Icon icon="ri:arrow-right-line" /></a>
		</div>

		<div class="folders-grid">
			{#each rootFolders as folder}
				<FolderCard
					name={folder.name}
					count={folder.count}
					compact={true}
					onclick={() => (window.location.href = `/home/files?folder=${folder.id}`)}
				/>
			{/each}
		</div>

		<div class="files-grid">
			{#each recentFiles as file}
				<FileCard
					name={file.name}
					size={formatSize(file.size)}
					date={formatTime(file.created_on)}
					type={file.type}
					encrypted={file.encrypted}
					onclick={() => (window.location.href = `/home/files`)}
				/>
			{/each}
			{#if recentFiles.length === 0 && rootFolders.length === 0}
				<div class="empty-state">
					<p>No recent files or folders.</p>
					<a href="/home/files" class="btn btn-primary">Go to Files</a>
				</div>
			{/if}
		</div>
	</div>
</div>

<style lang="scss">
	/* .dashboard uses the global .view rhythm (column, gap: space-6). */
	.upload-cta {
		white-space: nowrap;
		align-self: center;
	}

	.stats-section {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
		gap: var(--space-4);
	}

	.card {
		background: var(--bg-card);
		border: 1px solid var(--border-default);
		border-radius: var(--radius-md);
		box-shadow: var(--shadow-card);
		padding: var(--space-5);
		display: flex;
		flex-direction: column;
		gap: var(--space-4);

		.card-header {
			display: flex;
			align-items: center;
			gap: var(--space-3);
			h3 {
				margin: 0;
				font-size: var(--fs-lg);
				font-weight: var(--fw-semibold);
			}
		}

		.card-body {
			display: flex;
			flex-direction: column;
			gap: var(--space-3);
		}
	}

	.subscription-card {
		.plan-info {
			display: flex;
			align-items: center;
			justify-content: space-between;
			.plan-name {
				font-size: var(--fs-h3);
				font-weight: var(--fw-bold);
				color: var(--text-primary);
			}
			.plan-status {
				background: rgba(61, 220, 151, 0.15);
				color: var(--success);
				padding: var(--space-1) var(--space-3);
				border-radius: var(--radius-pill);
				font-size: var(--fs-xs);
				font-weight: var(--fw-semibold);
			}
		}
		.plan-desc {
			color: var(--text-secondary);
			font-size: var(--fs-sm);
			margin: 0;
		}
	}

	.subscription-card .btn,
	.empty-state .btn {
		align-self: flex-start;
		margin-top: var(--space-2);
	}

	.quota-card {
		.usage-info {
			display: flex;
			justify-content: space-between;
			align-items: baseline;
			.used {
				font-size: var(--fs-h3);
				font-weight: var(--fw-bold);
				color: var(--text-primary);
				font-family: var(--font-mono);
			}
			.total {
				color: var(--text-muted);
				font-size: var(--fs-sm);
			}
		}
		.progress-bar {
			height: 8px;
			background: var(--bg-input);
			border-radius: var(--radius-pill);
			overflow: hidden;
			.progress-fill {
				height: 100%;
				background: var(--accent-gradient);
				border-radius: var(--radius-pill);
				transition: width var(--dur) var(--ease);
			}
		}
		.free-space {
			color: var(--text-muted);
			font-size: var(--fs-xs);
			text-align: right;
			margin: 0;
		}
	}

	.recent-section {
		display: flex;
		flex-direction: column;
		gap: var(--space-4);

		.section-header {
			display: flex;
			justify-content: space-between;
			align-items: center;
			h2 {
				margin: 0;
				font-size: var(--fs-lg);
				font-weight: var(--fw-semibold);
			}
			.view-all {
				color: var(--primary);
				text-decoration: none;
				font-size: var(--fs-sm);
				font-weight: var(--fw-medium);
				display: flex;
				align-items: center;
				gap: var(--space-1);
				&:hover {
					color: var(--primary-hover);
				}
			}
		}
	}

	.folders-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
		gap: var(--space-3);
	}

	.files-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
		gap: var(--space-3);
	}

	.empty-state {
		grid-column: 1 / -1;
		text-align: center;
		padding: var(--space-5);
		background: var(--bg-card);
		border: 1px solid var(--border-default);
		border-radius: var(--radius-md);
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: var(--space-4);
		color: var(--text-secondary);

		.btn {
			align-self: center;
		}
	}

	@media (max-width: 600px) {
		.folders-grid,
		.files-grid {
			grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
			gap: var(--space-3);
		}
	}
</style>
