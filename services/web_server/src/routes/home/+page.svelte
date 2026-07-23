<script>
	import Icon from '@iconify/svelte';
	import { FrontendClient } from '$lib/frontendClient.js';
	import { browser } from '$app/environment';
	import { createQuery } from '@tanstack/svelte-query';
	import { Button, Progress, Skeleton, EmptyState } from '$lib/ui';

	let { data } = $props();

	// --- Data Fetching ---
	async function fetchFilesFn() {
		try {
			const res = await FrontendClient.get('/api/v1/sanctum/file/list');
			return res.data?.data?.files || res.data?.success?.data?.files || [];
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
	let loading = $derived(fetchFiles.isLoading || fetchFolders.isLoading);

	let allFiles = $derived(fetchFiles?.data || []);
	let allFolders = $derived(fetchFolders?.data || []);

	let recentFiles = $derived(
		[...allFiles]
			.sort((a, b) => new Date(b.created_on) - new Date(a.created_on))
			.slice(0, 8)
			.map((f) => ({ ...f, type: getFileType(f.mime) }))
	);

	let rootFolders = $derived(allFolders.slice(0, 6));

	let storage = $derived.by(() => {
		const fetched = fetchStorageStats?.data || { total: 0, used: 0, free: 0 };
		// Prefer the live stats total (base + all active subscriptions, incl. promos);
		// the session's totalAvailableSpace only carries one subscription and is stale.
		const total = fetched.total || data.user?.totalAvailableSpace || 0;
		return {
			used: fetched.used || 0,
			total,
			pct: total ? Math.min((fetched.used / total) * 100, 100) : 0
		};
	});

	let planName = $derived(data?.user?.subscription?.name || 'Free');
	let isPro = $derived(!!data?.user?.subscription);

	let greeting = $derived.by(() => {
		const h = new Date().getHours();
		if (h < 5) return 'Up late';
		if (h < 12) return 'Good morning';
		if (h < 18) return 'Good afternoon';
		return 'Good evening';
	});

	const today = new Date().toLocaleDateString(undefined, {
		weekday: 'long',
		month: 'long',
		day: 'numeric'
	});

	// --- Helpers ---
	function formatSize(bytes) {
		if (!bytes) return '0 B';
		const k = 1024;
		const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
		const i = Math.min(Math.floor(Math.log(bytes) / Math.log(k)), sizes.length - 1);
		return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
	}

	function relativeTime(dateString) {
		const then = new Date(dateString).getTime();
		if (!then) return '';
		const s = Math.max(0, (Date.now() - then) / 1000);
		if (s < 60) return 'just now';
		if (s < 3600) return `${Math.floor(s / 60)}m ago`;
		if (s < 86400) return `${Math.floor(s / 3600)}h ago`;
		if (s < 86400 * 7) return `${Math.floor(s / 86400)}d ago`;
		return new Date(dateString).toLocaleDateString(undefined, { month: 'short', day: 'numeric' });
	}

	function getFileType(mime) {
		if (mime?.includes('image')) return 'image';
		if (mime?.includes('video')) return 'video';
		if (mime?.includes('audio')) return 'audio';
		if (mime?.includes('pdf') || mime?.includes('document')) return 'doc';
		return 'file';
	}

	const typeIcons = {
		image: 'ri:image-line',
		video: 'ri:film-line',
		audio: 'ri:music-2-line',
		doc: 'ri:file-text-line',
		file: 'ri:file-3-line'
	};
</script>

<div class="view dashboard">
	<header class="dash-head">
		<div>
			<h1 class="greeting">
				{greeting}{data?.user?.username ? `, ${data.user.username}` : ''}.
			</h1>
			<p class="date">{today}</p>
		</div>
		<Button href="/home/files">
			<Icon icon="ri:upload-cloud-2-line" width="16" /> Upload
		</Button>
	</header>

	<!-- Stat band: one surface, four cells -->
	<div class="stat-band">
		<div class="cell storage-cell">
			<span class="label">Storage</span>
			<span class="value">{formatSize(storage.used)} <span class="sub">of {formatSize(storage.total)}</span></span>
			<Progress value={storage.pct} size="xs" tone={storage.pct > 90 ? 'warn' : 'accent'} label="Storage used" />
		</div>
		<div class="cell">
			<span class="label">Files</span>
			<span class="value">{loading ? '·' : allFiles.length}</span>
		</div>
		<div class="cell">
			<span class="label">Folders</span>
			<span class="value">{loading ? '·' : allFolders.length}</span>
		</div>
		<div class="cell">
			<span class="label">Plan</span>
			<span class="value">{planName}</span>
			<a class="cell-link" href="/home/subscription">
				{isPro ? 'Manage' : 'Upgrade'}
				<Icon icon="ri:arrow-right-line" width="12" />
			</a>
		</div>
	</div>

	{#if rootFolders.length > 0}
		<section class="block">
			<div class="block-head">
				<h2>Folders</h2>
			</div>
			<div class="folder-row">
				{#each rootFolders as folder (folder.id)}
					<a class="folder-pill" href={`/home/files?folder=${folder.id}`}>
						<Icon icon="ri:folder-3-line" width="15" />
						<span class="name">{folder.name}</span>
						{#if folder.count}<span class="count">{folder.count}</span>{/if}
					</a>
				{/each}
			</div>
		</section>
	{/if}

	<section class="block">
		<div class="block-head">
			<h2>Recent</h2>
			<a href="/home/files" class="view-all">
				View all <Icon icon="ri:arrow-right-line" width="13" />
			</a>
		</div>

		{#if loading}
			<div class="file-list">
				{#each Array(4) as _, i (i)}
					<div class="file-row skeleton-row">
						<Skeleton width="32px" height="32px" radius="var(--radius-sm)" />
						<Skeleton width="{160 + i * 40}px" height="0.9rem" />
					</div>
				{/each}
			</div>
		{:else if recentFiles.length === 0 && rootFolders.length === 0}
			<EmptyState icon="ri:inbox-2-line" title="Nothing here yet" line="Drop your first file and it will show up here, encrypted end to end.">
				<Button href="/home/files">Upload a file</Button>
			</EmptyState>
		{:else}
			<div class="file-list">
				{#each recentFiles as file (file.id)}
					<a class="file-row" href="/home/files">
						<span class="type-ic"><Icon icon={typeIcons[file.type]} width="16" /></span>
						<span class="name">
							{file.name}
							{#if file.encrypted}<Icon icon="ri:lock-2-line" width="12" class="lock" />{/if}
						</span>
						<span class="meta">{formatSize(file.size)}</span>
						<span class="meta time">{relativeTime(file.created_on)}</span>
					</a>
				{/each}
			</div>
		{/if}
	</section>
</div>

<style lang="scss">
	/* ---- header ---- */
	.dash-head {
		display: flex;
		align-items: flex-end;
		justify-content: space-between;
		gap: var(--space-4);
		flex-wrap: wrap;
	}

	.greeting {
		font-size: var(--fs-h2);
		font-weight: var(--fw-semibold);
		letter-spacing: var(--tracking-tight);
		margin: 0;
	}

	.date {
		margin: var(--space-1) 0 0;
		font-family: var(--font-mono);
		font-size: var(--fs-xs);
		color: var(--ink-faint);
	}

	/* ---- stat band ---- */
	.stat-band {
		display: grid;
		grid-template-columns: 1.6fr 1fr 1fr 1.2fr;
		background: var(--surface);
		border: 1px solid var(--edge);
		border-radius: var(--radius-md);
		overflow: hidden;
	}

	.cell {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		padding: var(--space-4) var(--space-5);
		min-width: 0;

		& + .cell {
			border-left: 1px solid var(--edge);
		}

		.label {
			font-size: var(--fs-xs);
			color: var(--ink-faint);
		}

		.value {
			font-family: var(--font-mono);
			font-size: var(--fs-lg);
			color: var(--ink);
			white-space: nowrap;
			overflow: hidden;
			text-overflow: ellipsis;

			.sub {
				font-size: var(--fs-xs);
				color: var(--ink-faint);
			}
		}

		.cell-link {
			display: inline-flex;
			align-items: center;
			gap: 3px;
			font-size: var(--fs-xs);
			color: var(--ink-mute);
			margin-top: auto;

			&:hover {
				color: var(--accent);
			}
		}
	}

	.storage-cell {
		:global(.progress) {
			margin-top: auto;
		}
	}

	/* ---- blocks ---- */
	.block {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
	}

	.block-head {
		display: flex;
		align-items: baseline;
		justify-content: space-between;

		h2 {
			font-size: var(--fs-body);
			font-weight: var(--fw-semibold);
			margin: 0;
		}

		.view-all {
			display: inline-flex;
			align-items: center;
			gap: 3px;
			font-size: var(--fs-sm);
			color: var(--ink-faint);

			&:hover {
				color: var(--ink);
			}
		}
	}

	/* ---- folder pills ---- */
	.folder-row {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-2);
	}

	.folder-pill {
		display: inline-flex;
		align-items: center;
		gap: var(--space-2);
		padding: var(--space-2) var(--space-3);
		background: var(--surface);
		border: 1px solid var(--edge);
		border-radius: var(--radius-md);
		color: var(--ink-mute);
		font-size: var(--fs-sm);
		max-width: 220px;
		transition:
			border-color var(--dur) var(--ease),
			color var(--dur) var(--ease),
			background var(--dur) var(--ease);

		.name {
			overflow: hidden;
			text-overflow: ellipsis;
			white-space: nowrap;
			color: var(--ink);
			font-weight: var(--fw-medium);
		}

		.count {
			font-family: var(--font-mono);
			font-size: var(--fs-xs);
			color: var(--ink-faint);
		}

		&:hover {
			background: var(--surface-hover);
			border-color: var(--edge-strong);
			color: var(--ink);
		}
	}

	/* ---- recent list ---- */
	.file-list {
		display: flex;
		flex-direction: column;
		background: var(--surface);
		border: 1px solid var(--edge);
		border-radius: var(--radius-md);
		overflow: hidden;
	}

	.file-row {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		padding: var(--space-3) var(--space-4);
		min-width: 0;
		transition: background var(--dur-fast) var(--ease);

		& + .file-row {
			border-top: 1px solid var(--edge);
		}

		&:hover {
			background: var(--surface-hover);
		}

		.type-ic {
			display: flex;
			align-items: center;
			justify-content: center;
			width: 32px;
			height: 32px;
			border: 1px solid var(--edge);
			border-radius: var(--radius-sm);
			color: var(--ink-faint);
			flex-shrink: 0;
		}

		.name {
			flex: 1;
			display: inline-flex;
			align-items: center;
			gap: var(--space-2);
			font-size: var(--fs-sm);
			font-weight: var(--fw-medium);
			color: var(--ink);
			white-space: nowrap;
			overflow: hidden;
			text-overflow: ellipsis;
			min-width: 0;

			:global(.lock) {
				color: var(--ink-faint);
				flex-shrink: 0;
			}
		}

		.meta {
			font-family: var(--font-mono);
			font-size: var(--fs-xs);
			color: var(--ink-mute);
			flex-shrink: 0;

			&.time {
				color: var(--ink-faint);
				width: 64px;
				text-align: right;
			}
		}

		&.skeleton-row {
			gap: var(--space-3);
		}
	}

	/* ---- responsive ---- */
	@media (max-width: 720px) {
		.stat-band {
			grid-template-columns: 1fr 1fr;
		}
		.cell + .cell {
			border-left: none;
		}
		.cell:nth-child(2) {
			border-left: 1px solid var(--edge);
		}
		.cell:nth-child(3),
		.cell:nth-child(4) {
			border-top: 1px solid var(--edge);
		}
		.cell:nth-child(4) {
			border-left: 1px solid var(--edge);
		}
		.file-row .meta.time {
			display: none;
		}
	}
</style>
