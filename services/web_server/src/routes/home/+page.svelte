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
		const total = data.user?.totalAvailableSpace || fetched.total;
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

<div class="dashboard-overview">
	<!-- Section 1: Subscription & Quota -->
	<div class="stats-section">
		<div class="card subscription-card">
			<div class="card-header">
				<Icon icon="ri:vip-crown-fill" width="24" color="#FFD700" />
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
				<button class="btn secondary" onclick={() => (window.location.href = '/home/subscription')}
					>Manage Subscription</button
				>
			</div>
		</div>

		<div class="card quota-card">
			<div class="card-header">
				<Icon icon="ri:hard-drive-2-line" width="24" color="var(--primary-color)" />
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
			<h2>Recent Activity</h2>
			<a href="/home/files" class="view-all">View All Files <Icon icon="ri:arrow-right-line" /></a>
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
					<a href="/home/files" class="btn primary">Go to Files</a>
				</div>
			{/if}
		</div>
	</div>
</div>

<style lang="scss">
	.dashboard-overview {
		padding: 32px;
		max-width: 1200px;
		margin: 0 auto;
		display: flex;
		flex-direction: column;
		gap: 40px;
	}

	.stats-section {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
		gap: 24px;
	}

	.card {
		background: var(--bg-card);
		border: 1px solid var(--border-default);
		border-radius: var(--radius-lg);
		padding: 24px;
		display: flex;
		flex-direction: column;
		gap: 16px;

		.card-header {
			display: flex;
			align-items: center;
			gap: 12px;
			h3 {
				margin: 0;
				font-size: 18px;
				font-weight: 600;
			}
		}

		.card-body {
			display: flex;
			flex-direction: column;
			gap: 12px;
		}
	}

	.subscription-card {
		.plan-info {
			display: flex;
			align-items: center;
			justify-content: space-between;
			.plan-name {
				font-size: 20px;
				font-weight: 700;
				color: var(--text-primary);
			}
			.plan-status {
				background: rgba(31, 122, 74, 0.2);
				color: #4ade80;
				padding: 4px 12px;
				border-radius: 12px;
				font-size: 12px;
				font-weight: 600;
			}
		}
		.plan-desc {
			color: var(--text-muted);
			font-size: 14px;
			margin: 0;
		}
	}

	.quota-card {
		.usage-info {
			display: flex;
			justify-content: space-between;
			align-items: baseline;
			.used {
				font-size: 24px;
				font-weight: 700;
				color: var(--text-primary);
			}
			.total {
				color: var(--text-muted);
				font-size: 14px;
			}
		}
		.progress-bar {
			height: 8px;
			background: var(--bg-input);
			border-radius: 4px;
			overflow: hidden;
			.progress-fill {
				height: 100%;
				background: var(--primary-color);
				border-radius: 4px;
			}
		}
		.free-space {
			color: var(--text-muted);
			font-size: 12px;
			text-align: right;
			margin: 0;
		}
	}

	.recent-section {
		display: flex;
		flex-direction: column;
		gap: 20px;

		.section-header {
			display: flex;
			justify-content: space-between;
			align-items: center;
			h2 {
				margin: 0;
				font-size: 20px;
			}
			.view-all {
				color: var(--primary-color);
				text-decoration: none;
				font-size: 14px;
				display: flex;
				align-items: center;
				gap: 4px;
				&:hover {
					text-decoration: underline;
				}
			}
		}
	}

	.folders-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
		gap: 16px;
	}

	.files-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
		gap: 16px;
	}

	.btn {
		padding: 10px 16px;
		border-radius: 8px;
		font-weight: 500;
		cursor: pointer;
		border: none;
		font-size: 14px;
		text-align: center;
		text-decoration: none;
		display: inline-block;

		&.primary {
			background: var(--primary-color);
			color: white;
		}
		&.secondary {
			background: transparent;
			border: 1px solid var(--border-default);
			color: var(--text-primary);
			&:hover {
				background: rgba(255, 255, 255, 0.05);
			}
		}
	}

	.empty-state {
		grid-column: 1 / -1;
		text-align: center;
		padding: 40px;
		background: var(--bg-card);
		border-radius: 12px;
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 16px;
		color: var(--text-muted);
	}
</style>
