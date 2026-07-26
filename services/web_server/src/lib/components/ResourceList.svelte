<script>
	/**
	 * The list shared by /home/shared and /home/starred.
	 *
	 * Both screens are the same object: a bordered list of resources with inline
	 * actions. Only the copy, the empty state, and whether share status is shown
	 * differ, so they take a `variant` rather than being two near-identical files.
	 */
	import Icon from '$lib/ui/Icon.svelte';
	import { FrontendClient } from '$lib/frontendClient.js';
	import { browser } from '$app/environment';
	import { createQuery, useQueryClient } from '@tanstack/svelte-query';
	import { toast } from '$lib/toast.js';
	import axios from 'axios';
	import { downloadFile } from '$lib/download.js';
	import { glyphForMime } from '$lib/ui/icons.js';
	import { copyShareLink } from '$lib/share.js';
	import Prompt from '$lib/ui/Prompt.svelte';
	import ShareModal from '$lib/components/ShareModal.svelte';

	let { variant = 'shared' } = $props();

	const COPY = {
		shared: {
			title: 'Shared',
			subtitle: "Files and folders you're sharing with others.",
			search: 'Search shared',
			emptyIcon: 'share',
			emptyTitle: 'Nothing shared yet',
			emptyLine: 'Share a file or folder and its link shows up here.',
			emptySearch: 'No shared items match your search.'
		},
		starred: {
			title: 'Starred',
			subtitle: 'The files and folders you want within reach.',
			search: 'Search starred',
			emptyIcon: 'star',
			emptyTitle: 'No starred items',
			emptyLine: 'Star a file or folder to keep it handy here.',
			emptySearch: 'No starred items match your search.'
		}
	};

	let copy = $derived(COPY[variant] ?? COPY.shared);
	let showShareStatus = $derived(variant === 'shared');

	const queryClient = useQueryClient();
	const filterKey = variant === 'shared' ? 'shared' : 'starred';

	const filesQuery = createQuery(() => ({
		queryKey: [`${filterKey}Files`],
		queryFn: async () => {
			try {
				const { data } = await FrontendClient.get('/api/v1/sanctum/file/list', {
					params: { [filterKey]: true }
				});
				return data?.data?.files || [];
			} catch (e) {
				console.error(`Error fetching ${filterKey} files:`, e);
				return [];
			}
		},
		enabled: browser
	}));

	const foldersQuery = createQuery(() => ({
		queryKey: [`${filterKey}Folders`],
		queryFn: async () => {
			try {
				const { data } = await FrontendClient.post('/api/v1/sanctum/folder/list', {
					[filterKey]: true
				});
				return data?.data?.folders || [];
			} catch (e) {
				console.error(`Error fetching ${filterKey} folders:`, e);
				return [];
			}
		},
		enabled: browser
	}));

	function invalidate() {
		queryClient.invalidateQueries({ queryKey: [`${filterKey}Files`] });
		queryClient.invalidateQueries({ queryKey: [`${filterKey}Folders`] });
		queryClient.invalidateQueries({ queryKey: ['fetchRecentFiles'] });
	}

	let loading = $derived(filesQuery.isLoading || foldersQuery.isLoading);
	let search = $state('');

	function matches(name) {
		const q = search.trim().toLowerCase();
		return !q || (name || '').toLowerCase().includes(q);
	}

	function formatSize(bytes) {
		if (!bytes) return '0 B';
		const k = 1024;
		const s = ['B', 'KB', 'MB', 'GB', 'TB'];
		const i = Math.min(Math.floor(Math.log(bytes) / Math.log(k)), s.length - 1);
		return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + s[i];
	}

	/** `1.8 MB · 3 downloads · password`: only the parts that are true. */
	function metaFor(item, isFolder) {
		const bits = [];
		bits.push(isFolder ? `${item.count ?? 0} items` : formatSize(item.size));
		if (showShareStatus) {
			const n = Number(item.link_downloads ?? 0);
			bits.push(`${n} download${n === 1 ? '' : 's'}`);
			if (item.share_password_protected || item.share_has_password) bits.push('password');
		}
		return bits.join(' · ');
	}

	/** Active / expires in 2d / One-time, or nothing when sharing is off. */
	function badgeFor(item) {
		if (!showShareStatus) return null;
		const type = item.share_type || 'off';
		if (type === 'off') return null;
		if (type === 'once') return { label: 'One-time', tone: 'neutral' };

		const expires = item.share_expires_at ? new Date(item.share_expires_at).getTime() : null;
		if (expires) {
			const days = Math.ceil((expires - Date.now()) / 86400_000);
			if (days <= 0) return { label: 'Expired', tone: 'danger' };
			if (days <= 7) return { label: `expires in ${days}d`, tone: 'warn' };
		}
		return { label: 'Active', tone: 'ok' };
	}

	let rows = $derived.by(() => {
		const folders = (foldersQuery.data || [])
			.filter((f) => matches(f.name))
			.map((f) => ({
				...f,
				kind: 'folder',
				glyph: 'folder',
				meta: metaFor(f, true),
				badge: badgeFor(f)
			}));
		const files = (filesQuery.data || [])
			.filter((f) => matches(f.name))
			.map((f) => ({
				...f,
				kind: 'file',
				glyph: glyphForMime(f.mime, f.name),
				meta: metaFor(f, false),
				badge: badgeFor(f)
			}));
		return [...folders, ...files];
	});

	let isEmpty = $derived(!loading && rows.length === 0);

	// ---- actions -----------------------------------------------------------
	let passwordPrompt = $state({ open: false, file: null });

	function handleDownload(item) {
		if (item.kind === 'folder') {
			toast.info('Open the folder to download it', 'Folder zips are built from the Files screen.');
			return;
		}
		if (item.encrypted) passwordPrompt = { open: true, file: item };
		else downloadFile(item);
	}

	function submitPassword(pw) {
		const f = passwordPrompt.file;
		passwordPrompt = { open: false, file: null };
		if (f && pw) downloadFile(f, { password: pw });
	}

	let shareItem = $state(null);

	async function toggleStar(item) {
		const next = !item.starred;
		try {
			await axios.post(`/api/v1/sanctum/${item.kind}/star`, {
				[item.kind === 'file' ? 'file_id' : 'folder_id']: item.id,
				starred: next
			});
			toast.success(next ? 'Starred' : 'Removed from starred', item.name);
			invalidate();
		} catch {
			toast.error('Could not update that item');
		}
	}
</script>

<div class="view">
	<header class="head">
		<div class="head-text">
			<h1>{copy.title}</h1>
			<span class="sub">{copy.subtitle}</span>
		</div>
		<div class="search">
			<span class="search-glyph"><Icon name="search" size={15} /></span>
			<input type="text" placeholder={copy.search} bind:value={search} spellcheck="false" />
		</div>
	</header>

	{#if loading}
		<div class="list">
			{#each Array(5) as _, i (i)}
				<div class="row">
					<span class="tile sk-tile"></span>
					<span class="sk" style="width:{170 + i * 30}px"></span>
				</div>
			{/each}
		</div>
	{:else if isEmpty}
		<div class="empty">
			<Icon name={copy.emptyIcon} size={34} stroke={1.2} />
			<div class="empty-text">
				<span class="empty-title">{copy.emptyTitle}</span>
				<span class="empty-line">{search ? copy.emptySearch : copy.emptyLine}</span>
			</div>
			{#if search}
				<button type="button" class="text-btn" onclick={() => (search = '')}>Clear search</button>
			{/if}
		</div>
	{:else}
		<div class="list">
			{#each rows as r (r.kind + r.id)}
				<div class="row">
					<span class="tile"><Icon name={r.glyph} size={16} /></span>

					<div class="text">
						<div class="name-row">
							<span class="name" title={r.name}>{r.name}</span>
							{#if r.encrypted}
								<span class="ind"><Icon name="lock" size={13} /></span>
							{/if}
							{#if r.starred}
								<span class="ind"><Icon name="star-fill" size={13} /></span>
							{/if}
						</div>
						<span class="meta">{r.meta}</span>
					</div>

					{#if r.badge}
						<span class="badge {r.badge.tone}">{r.badge.label}</span>
					{/if}

					<div class="actions">
						<button
							type="button"
							class="act"
							title="Copy link"
							aria-label="Copy link to {r.name}"
							onclick={() => copyShareLink(r, r.kind)}
						>
							<Icon name="link" size={15} />
						</button>
						<button
							type="button"
							class="act"
							title="Share settings"
							aria-label="Share settings for {r.name}"
							onclick={() => (shareItem = r)}
						>
							<Icon name="share" size={15} />
						</button>
						<button
							type="button"
							class="act"
							title="Download"
							aria-label="Download {r.name}"
							onclick={() => handleDownload(r)}
						>
							<Icon name="download" size={15} />
						</button>
						<button
							type="button"
							class="act"
							title={r.starred ? 'Unstar' : 'Star'}
							aria-label={r.starred ? `Unstar ${r.name}` : `Star ${r.name}`}
							onclick={() => toggleStar(r)}
						>
							<Icon name="star" size={15} filled={r.starred} />
						</button>
					</div>
				</div>
			{/each}
		</div>
	{/if}
</div>

<Prompt
	open={passwordPrompt.open}
	title="Password needed"
	message="This file is encrypted. Enter its password to download."
	placeholder="Password"
	submitLabel="Download"
	onsubmit={submitPassword}
	onclose={() => (passwordPrompt = { open: false, file: null })}
/>

{#if shareItem}
	<ShareModal item={shareItem} on:close={() => { shareItem = null; invalidate(); }} />
{/if}

<style lang="scss">
	.view {
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

	.search {
		position: relative;
		flex: 0 1 280px;

		input {
			width: 100%;
			height: 32px;
			padding: 0 0.75rem 0 2rem;
			border-radius: var(--radius-sm);
			background: var(--surface);
			border: 1px solid var(--edge);
			color: var(--ink);
			font: inherit;
			font-size: var(--fs-sm);
			outline: none;
			transition:
				border-color var(--dur-fast) var(--ease),
				box-shadow var(--dur-fast) var(--ease);

			&::placeholder {
				color: var(--ink-faint);
			}
			&:focus {
				border-color: var(--accent);
				box-shadow: 0 0 0 3px var(--focus-ring);
			}
		}
	}

	.search-glyph {
		position: absolute;
		left: 0.625rem;
		top: 50%;
		transform: translateY(-50%);
		color: var(--ink-faint);
		pointer-events: none;
	}

	.list {
		border: 1px solid var(--edge);
		border-radius: var(--radius-md);
		background: var(--surface);
		overflow: hidden;
	}

	.row {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		padding: 0.75rem 1rem;
		border-bottom: 1px solid var(--edge);
		transition: background var(--dur-fast) var(--ease);

		&:last-child {
			border-bottom: 0;
		}
		&:hover {
			background: var(--surface-hover);
		}
	}

	.tile {
		flex: 0 0 auto;
		display: grid;
		place-items: center;
		width: 34px;
		height: 34px;
		border-radius: 8px;
		background: var(--tint-soft);
		color: var(--ink-mute);
	}

	.text {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 0.125rem;
	}

	.name-row {
		display: flex;
		align-items: center;
		gap: 0.4375rem;
		min-width: 0;
	}

	.name {
		font-size: 0.875rem;
		font-weight: var(--fw-medium);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.ind {
		flex: 0 0 auto;
		display: grid;
		place-items: center;
		color: var(--ink-faint);
	}

	.meta {
		font-family: var(--font-mono);
		font-size: var(--fs-xs);
		color: var(--ink-faint);
	}

	.badge {
		flex: 0 0 auto;
		display: inline-flex;
		align-items: center;
		height: 20px;
		padding-inline: 0.4375rem;
		border-radius: var(--radius-sm);
		font-size: var(--fs-xs);
		font-weight: var(--fw-medium);

		&.ok {
			background: var(--ok-soft);
			color: var(--ok);
		}
		&.warn {
			background: var(--warn-soft);
			color: var(--warn);
		}
		&.danger {
			background: var(--danger-soft);
			color: var(--danger);
		}
		&.neutral {
			background: var(--tint-softer);
			color: var(--ink-mute);
		}
	}

	.actions {
		flex: 0 0 auto;
		display: flex;
		align-items: center;
		gap: 0.125rem;
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
	}

	.text-btn {
		border: 0;
		background: none;
		font: inherit;
		font-size: var(--fs-sm);
		color: var(--accent);
		cursor: pointer;
	}

	.sk {
		display: block;
		height: 0.9rem;
		border-radius: var(--radius-sm);
		background: var(--tint-softer);
	}

	.sk-tile {
		background: var(--tint-softer);
		color: transparent;
	}

	@media (max-width: 720px) {
		.head {
			flex-direction: column;
			align-items: stretch;
		}
		.search {
			flex: 1 1 auto;
		}
		.meta {
			font-size: 0.6875rem;
		}
	}
</style>
