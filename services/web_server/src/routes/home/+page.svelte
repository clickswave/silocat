<script>
	import Icon from '$lib/ui/Icon.svelte';
	import { FrontendClient } from '$lib/frontendClient.js';
	import { browser } from '$app/environment';
	import { createQuery } from '@tanstack/svelte-query';
	import { glyphForMime } from '$lib/ui/icons.js';
	import { copyShareLink } from '$lib/share.js';

	let { data } = $props();

	// --- data ---------------------------------------------------------------
	const fetchFiles = createQuery(() => ({
		queryKey: ['fetchRecentFiles'],
		queryFn: async () => {
			try {
				const res = await FrontendClient.get('/api/v1/sanctum/file/list');
				return res.data?.data?.files || res.data?.success?.data?.files || [];
			} catch (e) {
				console.error('Error fetching recent files:', e);
				return [];
			}
		},
		enabled: browser
	}));

	const fetchFolders = createQuery(() => ({
		queryKey: ['fetchRootFolders'],
		queryFn: async () => {
			try {
				const res = await FrontendClient.post('/api/v1/sanctum/folder/list', { parent_id: null });
				return res.data?.data?.folders || [];
			} catch (e) {
				console.error('Error fetching folders:', e);
				return [];
			}
		},
		enabled: browser
	}));

	const fetchStorageStats = createQuery(() => ({
		queryKey: ['fetchStorageStats'],
		queryFn: async () => {
			try {
				const { data: d } = await FrontendClient.get('/api/v1/sanctum/user/storage');
				return d?.success || { total: 0, used: 0, free: 0 };
			} catch (e) {
				console.error('Error fetching storage stats:', e);
				return { total: 0, used: 0, free: 0 };
			}
		},
		enabled: browser
	}));

	let loading = $derived(fetchFiles.isLoading || fetchFolders.isLoading);
	let allFiles = $derived(fetchFiles?.data || []);
	let allFolders = $derived(fetchFolders?.data || []);

	let recentFiles = $derived(
		[...allFiles]
			.sort((a, b) => new Date(b.created_on) - new Date(a.created_on))
			.slice(0, 8)
			.map((f) => ({ ...f, glyph: glyphForMime(f.mime, f.name) }))
	);

	let rootFolders = $derived(allFolders.slice(0, 6));

	let storage = $derived.by(() => {
		const fetched = fetchStorageStats?.data || { total: 0, used: 0, free: 0 };
		// Prefer the live stats total (base + all active subscriptions, incl. promos);
		// the session's totalAvailableSpace only carries one subscription and goes stale.
		const total = fetched.total || data.user?.totalAvailableSpace || 0;
		return {
			used: fetched.used || 0,
			total,
			pct: total ? Math.min((fetched.used / total) * 100, 100) : 0
		};
	});

	let planName = $derived(data?.user?.subscription?.name || 'Free');
	let isPaid = $derived(!!data?.user?.subscription);
	let isEmpty = $derived(!loading && allFiles.length === 0 && allFolders.length === 0);

	// --- "Get set up" -------------------------------------------------------
	// Steps are derived from real account state rather than a stored checklist,
	// so the card can never disagree with what the user has actually done. Only
	// the dismissal is remembered, and that is a per-browser preference.
	const ONBOARD_KEY = 'silocat-onboarding-dismissed';

	let dismissed = $state(browser ? localStorage.getItem(ONBOARD_KEY) === '1' : false);

	function dismissOnboarding() {
		dismissed = true;
		try {
			localStorage.setItem(ONBOARD_KEY, '1');
		} catch {
			/* private mode: dismissal lasts for this session only */
		}
	}

	let steps = $derived([
		{ label: 'Upload your first file', done: allFiles.length > 0, href: '/home/files?upload=1' },
		{
			label: 'Share your first link',
			done: allFiles.some((f) => f.public_access || f.share_type === 'public'),
			href: '/home/files'
		},
		{ label: 'Verify your email', done: data?.user?.email_verified === true, href: '/home/settings' }
	]);

	let doneCount = $derived(steps.filter((s) => s.done).length);
	let showOnboarding = $derived(!dismissed && !loading && doneCount < steps.length);

	// --- helpers ------------------------------------------------------------
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
		if (s < 60) return 'now';
		if (s < 3600) return `${Math.floor(s / 60)}m`;
		if (s < 86400) return `${Math.floor(s / 3600)}h`;
		if (s < 86400 * 7) return `${Math.floor(s / 86400)}d`;
		return new Date(dateString).toLocaleDateString(undefined, { month: 'short', day: 'numeric' });
	}

	let greeting = $derived.by(() => {
		const h = new Date().getHours();
		if (h < 5) return 'Up late';
		if (h < 12) return 'Good morning';
		if (h < 18) return 'Good afternoon';
		return 'Good evening';
	});

	const today = new Date().toLocaleDateString(undefined, {
		weekday: 'long',
		day: 'numeric',
		month: 'long',
		year: 'numeric'
	});
</script>

<div class="dash">
	<header class="head">
		<div class="head-text">
			<h1>{greeting}{data?.user?.username ? `, ${data.user.username}` : ''}.</h1>
			<span class="date">{today}</span>
		</div>
		<a href="/home/files?upload=1" class="upload-btn">
			<Icon name="upload" size={16} />
			Upload
		</a>
	</header>

	{#if showOnboarding}
		<section class="onboard">
			<div class="onboard-head">
				<div class="onboard-text">
					<span class="onboard-title">Get set up</span>
					<span class="onboard-sub">
						{doneCount} of {steps.length} done · takes about a minute
					</span>
				</div>
				<button type="button" class="dismiss" onclick={dismissOnboarding}>Dismiss</button>
			</div>
			<div class="steps">
				{#each steps as step, i (step.label)}
					{#if step.done}
						<div class="step done">
							<span class="marker ok"><Icon name="check" size={12} stroke={2.4} /></span>
							<span class="step-label">{step.label}</span>
						</div>
					{:else}
						<a class="step" href={step.href}>
							<span class="marker">{i + 1}</span>
							<span class="step-label">{step.label}</span>
						</a>
					{/if}
				{/each}
			</div>
		</section>
	{/if}

	{#if isEmpty}
		<section class="empty">
			<Icon name="upload-lg" size={36} />
			<div class="empty-text">
				<span class="empty-title">Nothing here yet</span>
				<span class="empty-line">
					Drop your first file and it will show up here, encrypted end to end.
				</span>
			</div>
			<a href="/home/files?upload=1" class="upload-btn">Upload a file</a>
		</section>
	{:else}
		<section class="stat-band">
			<div class="cell">
				<span class="cell-label">Storage</span>
				<span class="cell-value mono">
					{formatSize(storage.used)} <span class="faint">of {formatSize(storage.total)}</span>
				</span>
				<div class="meter">
					<div class="fill" class:warn={storage.pct > 90} style="width:{storage.pct}%"></div>
				</div>
			</div>
			<div class="cell">
				<span class="cell-label">Files</span>
				<span class="cell-value mono big">{loading ? '·' : allFiles.length}</span>
			</div>
			<div class="cell">
				<span class="cell-label">Folders</span>
				<span class="cell-value mono big">{loading ? '·' : allFolders.length}</span>
			</div>
			<div class="cell">
				<span class="cell-label">Plan</span>
				<span class="cell-value">{planName}</span>
				<a class="cell-link" href="/home/billing">{isPaid ? 'Manage' : 'Upgrade'} →</a>
			</div>
		</section>

		{#if rootFolders.length > 0}
			<section class="block">
				<span class="block-label">Folders</span>
				<div class="pills">
					{#each rootFolders as folder (folder.id)}
						<a class="pill" href={`/home/files?folder=${folder.id}`}>
							<Icon name="folder" size={15} />
							<span class="pill-name">{folder.name}</span>
							{#if folder.count}<span class="pill-count">{folder.count}</span>{/if}
						</a>
					{/each}
				</div>
			</section>
		{/if}

		<section class="block">
			<div class="block-head">
				<span class="block-label">Recent</span>
				<a href="/home/files" class="view-all">View all →</a>
			</div>

			<div class="recent">
				{#if loading}
					{#each Array(6) as _, i (i)}
						<div class="rrow skeleton">
							<span class="sk sk-glyph"></span>
							<span class="sk sk-name" style="width:{140 + i * 30}px"></span>
						</div>
					{/each}
				{:else}
					{#each recentFiles as file (file.id)}
						<div class="rrow">
							<span class="rglyph"><Icon name={file.glyph} size={16} /></span>
							<span class="rname">{file.name}</span>
							{#if file.encrypted}
								<span class="rlock"><Icon name="lock" size={13} /></span>
							{/if}
							<div class="ractions">
								<button
									type="button"
									class="ract"
									title="Copy link"
									aria-label="Copy share link"
									onclick={() => copyShareLink(file, 'file')}
								>
									<Icon name="link" size={14} />
								</button>
								<a
									class="ract"
									href="/home/files"
									title="Share"
									aria-label="Share settings"
								>
									<Icon name="share" size={14} />
								</a>
							</div>
							<span class="rmeta">{formatSize(file.size)}</span>
							<span class="rmeta narrow">{relativeTime(file.created_on)}</span>
						</div>
					{/each}
				{/if}
			</div>
		</section>
	{/if}
</div>

<style lang="scss">
	.dash {
		display: flex;
		flex-direction: column;
		gap: 1.25rem;
	}

	/* ---- header ---- */
	.head {
		display: flex;
		align-items: flex-start;
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

	.date {
		font-size: var(--fs-sm);
		color: var(--ink-faint);
	}

	.upload-btn {
		display: flex;
		align-items: center;
		gap: 0.4375rem;
		height: 36px;
		padding-inline: 1rem;
		border-radius: var(--radius-md);
		background: var(--accent);
		color: #fff;
		font-size: var(--fs-sm);
		font-weight: var(--fw-medium);
		text-decoration: none;
		flex: 0 0 auto;
		transition: background var(--dur-fast) var(--ease);

		&:hover {
			background: var(--accent-hover);
			color: #fff;
		}
	}

	/* ---- onboarding ---- */
	.onboard {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
		padding: 1rem;
		border: 1px solid var(--edge);
		border-radius: var(--radius-md);
		background: var(--surface);
	}

	.onboard-head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-4);
	}

	.onboard-text {
		display: flex;
		flex-direction: column;
		gap: 0.125rem;
	}

	.onboard-title {
		font-size: 0.9375rem;
		font-weight: var(--fw-semibold);
		letter-spacing: var(--tracking-tight);
	}

	.onboard-sub {
		font-size: var(--fs-sm);
		color: var(--ink-faint);
	}

	.dismiss {
		border: 0;
		background: none;
		font: inherit;
		font-size: var(--fs-xs);
		color: var(--ink-faint);
		padding: var(--space-1) var(--space-2);
		border-radius: var(--radius-sm);
		cursor: pointer;
		transition:
			background var(--dur-fast) var(--ease),
			color var(--dur-fast) var(--ease);

		&:hover {
			background: var(--tint-soft);
			color: var(--ink);
		}
	}

	.steps {
		display: grid;
		grid-template-columns: repeat(3, 1fr);
		gap: var(--space-2);
	}

	.step {
		display: flex;
		align-items: center;
		gap: 0.625rem;
		padding: 0.625rem 0.75rem;
		border: 1px solid var(--edge);
		border-radius: 8px;
		color: var(--ink);
		text-decoration: none;
		transition:
			background var(--dur-fast) var(--ease),
			border-color var(--dur-fast) var(--ease);

		&:not(.done):hover {
			background: var(--surface-hover);
			border-color: var(--edge-strong);
		}

		&.done {
			opacity: 0.6;

			.step-label {
				text-decoration: line-through;
				text-decoration-color: var(--ink-faint);
			}
		}
	}

	.marker {
		display: grid;
		place-items: center;
		width: 20px;
		height: 20px;
		flex: 0 0 auto;
		border-radius: var(--radius-full);
		border: 1px solid var(--edge-strong);
		color: var(--ink-faint);
		font-size: 0.6875rem;
		font-weight: var(--fw-semibold);

		&.ok {
			border-color: transparent;
			background: var(--ok-soft);
			color: var(--ok);
		}
	}

	.step-label {
		font-size: var(--fs-sm);
	}

	/* ---- stat band ---- */
	.stat-band {
		display: grid;
		grid-template-columns: repeat(4, 1fr);
		border: 1px solid var(--edge);
		border-radius: var(--radius-md);
		background: var(--surface);
	}

	.cell {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		padding: 1rem;
		border-right: 1px solid var(--edge);

		&:last-child {
			border-right: 0;
		}
	}

	.cell-label {
		font-size: var(--fs-xs);
		color: var(--ink-faint);
	}

	.cell-value {
		font-size: 0.9375rem;
		font-weight: var(--fw-medium);

		&.mono {
			font-family: var(--font-mono);
			font-weight: var(--fw-regular);
		}
		&.big {
			font-size: 1.25rem;
			font-weight: var(--fw-medium);
		}
	}

	.faint {
		color: var(--ink-faint);
	}

	.meter {
		height: 4px;
		border-radius: var(--radius-full);
		background: var(--tint-softer);
		overflow: hidden;
		margin-top: 0.125rem;
	}

	.fill {
		height: 100%;
		border-radius: var(--radius-full);
		background: var(--accent);

		&.warn {
			background: var(--warn);
		}
	}

	.cell-link {
		font-size: var(--fs-xs);
		font-weight: var(--fw-medium);
		color: var(--ink-mute);
		text-decoration: none;
		transition: color var(--dur-fast) var(--ease);

		&:hover {
			color: var(--ink);
		}
	}

	/* ---- blocks ---- */
	.block {
		display: flex;
		flex-direction: column;
		gap: 0.625rem;
	}

	.block-head {
		display: flex;
		align-items: baseline;
		justify-content: space-between;
		padding-inline: 0.125rem;
	}

	.block-label {
		font-size: var(--fs-xs);
		font-weight: var(--fw-medium);
		color: var(--ink-faint);
		text-transform: uppercase;
		letter-spacing: 0.06em;
		padding-inline: 0.125rem;
	}

	.view-all {
		font-size: var(--fs-sm);
		color: var(--ink-mute);
		text-decoration: none;
		transition: color var(--dur-fast) var(--ease);

		&:hover {
			color: var(--ink);
		}
	}

	.pills {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-2);
	}

	.pill {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		height: 36px;
		padding-inline: 0.875rem;
		border: 1px solid var(--edge);
		border-radius: var(--radius-md);
		background: var(--surface);
		color: var(--ink-mute);
		text-decoration: none;
		transition:
			background var(--dur-fast) var(--ease),
			border-color var(--dur-fast) var(--ease);

		&:hover {
			background: var(--surface-hover);
			border-color: var(--edge-strong);
		}
	}

	.pill-name {
		font-size: var(--fs-sm);
		font-weight: var(--fw-medium);
		color: var(--ink);
	}

	.pill-count {
		font-family: var(--font-mono);
		font-size: var(--fs-xs);
		color: var(--ink-faint);
	}

	/* ---- recent ---- */
	.recent {
		border: 1px solid var(--edge);
		border-radius: var(--radius-md);
		background: var(--surface);
		overflow: hidden;
	}

	.rrow {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		padding: 0.5625rem 1rem;
		border-bottom: 1px solid var(--edge);
		transition: background var(--dur-fast) var(--ease);

		&:last-child {
			border-bottom: 0;
		}
		&:hover {
			background: var(--surface-hover);
		}
	}

	.rglyph,
	.rlock {
		flex: 0 0 auto;
		display: grid;
		place-items: center;
		color: var(--ink-mute);
	}
	.rlock {
		color: var(--ink-faint);
	}

	.rname {
		flex: 1;
		min-width: 0;
		font-size: var(--fs-sm);
		font-weight: var(--fw-medium);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.ractions {
		flex: 0 0 auto;
		display: flex;
		align-items: center;
		gap: 0.125rem;
	}

	.ract {
		width: 26px;
		height: 26px;
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
	}

	.rmeta {
		flex: 0 0 72px;
		text-align: right;
		font-family: var(--font-mono);
		font-size: var(--fs-xs);
		color: var(--ink-faint);

		&.narrow {
			flex-basis: 56px;
		}
	}

	/* ---- skeleton + empty ---- */
	.skeleton {
		pointer-events: none;
	}

	.sk {
		display: block;
		height: 0.9rem;
		border-radius: var(--radius-sm);
		background: var(--tint-softer);
	}

	.sk-glyph {
		width: 16px;
		height: 16px;
		flex: 0 0 auto;
	}

	.empty {
		flex: 1;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 0.875rem;
		border: 1px dashed var(--edge-strong);
		border-radius: var(--radius-md);
		background: var(--surface);
		padding: 4rem 1rem;
		text-align: center;
		color: var(--ink-faint);
	}

	.empty-text {
		display: flex;
		flex-direction: column;
		gap: 0.375rem;
		max-width: 38ch;
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
		line-height: var(--lh-normal);
	}

	@media (max-width: 900px) {
		.stat-band {
			grid-template-columns: repeat(2, 1fr);
		}
		.cell:nth-child(2) {
			border-right: 0;
		}
		.cell:nth-child(1),
		.cell:nth-child(2) {
			border-bottom: 1px solid var(--edge);
		}
		.steps {
			grid-template-columns: 1fr;
		}
	}

	@media (max-width: 640px) {
		.ractions {
			display: none;
		}
	}
</style>
