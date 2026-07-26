<script>
	import { page } from '$app/stores';
	import { browser } from '$app/environment';
	import { createQuery } from '@tanstack/svelte-query';
	import Icon from '$lib/ui/Icon.svelte';
	import Avatar from '$lib/ui/Avatar.svelte';
	import { FrontendClient } from '$lib/frontendClient.js';
	import { theme, toggleTheme } from '$lib/theme.js';
	import { sidebarCollapsed, toggleSidebar } from '$lib/stores/sidebar.js';

	// `open` only matters on mobile, where the sidebar is an off-canvas drawer.
	let { open = false, onclose } = $props();

	const primary = [
		{ icon: 'home', label: 'Home', href: '/home' },
		{ icon: 'files', label: 'Files', href: '/home/files' },
		{ icon: 'share', label: 'Shared', href: '/home/shared' },
		{ icon: 'star', label: 'Starred', href: '/home/starred' },
		{ icon: 'trash', label: 'Trash', href: '/home/trash' }
	];

	const secondary = [
		{ icon: 'billing', label: 'Billing', href: '/home/billing' },
		{ icon: 'settings', label: 'Settings', href: '/home/settings' },
		{ icon: 'support', label: 'Support', href: '/home/support' }
	];

	let user = $derived($page.data.user || {});
	// The drawer is always full width on mobile, so collapse only applies when
	// the rail is in flow.
	let collapsed = $derived($sidebarCollapsed && !open);

	// Shared query key with the files page + dashboard: uploads/deletes there call
	// queryClient.invalidateQueries(['fetchStorageStats']), which refetches this and
	// keeps the sidebar meter live without a manual page refresh.
	const storageQuery = createQuery(() => ({
		queryKey: ['fetchStorageStats'],
		queryFn: async () => {
			const { data } = await FrontendClient.get('/api/v1/sanctum/user/storage');
			return data?.success || { used: 0, total: 0 };
		},
		enabled: browser
	}));

	let storage = $derived({
		used: storageQuery.data?.used || 0,
		total: storageQuery.data?.total || user.totalAvailableSpace || 0
	});

	let usedPct = $derived(storage.total ? Math.min((storage.used / storage.total) * 100, 100) : 0);

	function formatSize(bytes) {
		if (!bytes) return '0 B';
		const k = 1024;
		const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
		const i = Math.min(Math.floor(Math.log(bytes) / Math.log(k)), sizes.length - 1);
		return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
	}

	let isActive = (href) =>
		href === '/home' ? $page.url.pathname === '/home' : $page.url.pathname.startsWith(href);
</script>

<aside class="aside" class:open class:collapsed>
	<div class="head" class:centered={collapsed}>
		<a href="/home" class="brand" title="Home">
			<img src="/silocat-logo.png" alt="" width="24" height="24" />
			{#if !collapsed}<span class="wordmark">silocat</span>{/if}
		</a>
		{#if !collapsed}
			<button
				type="button"
				class="ghost-btn hide-mobile"
				aria-label="Collapse sidebar"
				title="Collapse sidebar"
				onclick={toggleSidebar}
			>
				<Icon name="chevrons-left" size={15} />
			</button>
		{/if}
		<button class="nav-close" onclick={onclose} aria-label="Close menu">
			<Icon name="close" size={18} />
		</button>
	</div>

	{#if collapsed}
		<div class="expand-row">
			<button
				type="button"
				class="ghost-btn"
				aria-label="Expand sidebar"
				title="Expand sidebar"
				onclick={toggleSidebar}
			>
				<Icon name="chevrons-left" size={15} style="transform:rotate(180deg)" />
			</button>
		</div>
	{/if}

	<div class="upload-wrap">
		<a href="/home/files?upload=1" class="upload" title="Upload">
			<Icon name="upload" size={16} />
			{#if !collapsed}<span>Upload</span>{/if}
		</a>
	</div>

	<nav class="nav" aria-label="App">
		{#each primary as item (item.href)}
			<a
				href={item.href}
				class="item"
				class:active={isActive(item.href)}
				class:centered={collapsed}
				title={item.label}
			>
				<Icon name={item.icon} size={17} />
				{#if !collapsed}<span>{item.label}</span>{/if}
			</a>
		{/each}

		<div class="divider"></div>

		{#each secondary as item (item.href)}
			<a
				href={item.href}
				class="item"
				class:active={isActive(item.href)}
				class:centered={collapsed}
				title={item.label}
			>
				<Icon name={item.icon} size={17} />
				{#if !collapsed}<span>{item.label}</span>{/if}
			</a>
		{/each}
	</nav>

	<div class="foot">
		{#if !collapsed}
			<div class="storage">
				<div class="meter">
					<div
						class="fill"
						class:warn={usedPct > 90}
						style="width:{usedPct}%"
						role="progressbar"
						aria-label="Storage used"
						aria-valuenow={Math.round(usedPct)}
						aria-valuemin="0"
						aria-valuemax="100"
					></div>
				</div>
				<div class="storage-row">
					<span class="storage-text">
						{formatSize(storage.used)} / {formatSize(storage.total)}
					</span>
					<a href="/home/billing" class="get-more">Get more →</a>
				</div>
			</div>
		{/if}

		<div class="user-row" class:centered={collapsed}>
			<Avatar src={user.profile_image} name={user.username} size={28} />
			{#if !collapsed}
				<div class="details">
					<span class="username">{user.username || 'User'}</span>
					<span class="email" title={user.email}>{user.email || ''}</span>
				</div>
				<button
					type="button"
					class="ghost-btn sm"
					aria-label={$theme === 'dark' ? 'Switch to light theme' : 'Switch to dark theme'}
					title="Toggle theme"
					onclick={toggleTheme}
				>
					<Icon name={$theme === 'dark' ? 'sun' : 'moon'} size={15} />
				</button>
				<form action="/auth/logout" method="POST" class="logout-form">
					<button type="submit" class="ghost-btn sm" title="Sign out" aria-label="Sign out">
						<Icon name="logout" size={15} />
					</button>
				</form>
			{/if}
		</div>
	</div>
</aside>

<style lang="scss">
	.aside {
		display: flex;
		flex-direction: column;
		width: 240px;
		flex: 0 0 auto;
		height: 100vh;
		background: var(--surface);
		border-right: 1px solid var(--edge);
		overflow: hidden;
		font-family: var(--font-sans);
		transition: width var(--dur) var(--ease);
		z-index: 100;

		&.collapsed {
			width: 64px;
		}
	}

	.head {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		padding: 0.875rem 0.875rem 0.75rem;

		&.centered {
			justify-content: center;
		}
	}

	.brand {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		color: var(--ink);
		text-decoration: none;
		min-width: 0;
		flex: 1;

		img {
			width: 24px;
			height: 24px;
			border-radius: var(--radius-sm);
			display: block;
			flex: 0 0 auto;
		}
	}

	.head.centered .brand {
		flex: 0 0 auto;
	}

	.wordmark {
		font-size: 0.9375rem;
		font-weight: var(--fw-semibold);
		letter-spacing: var(--tracking-tight);
		color: var(--ink);
	}

	.ghost-btn {
		flex: 0 0 auto;
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
			background: var(--nav-hover);
			color: var(--ink);
		}
		&.sm {
			width: 28px;
			height: 28px;
		}
	}

	.expand-row {
		display: flex;
		justify-content: center;
		padding: 0 0 0.375rem;
	}

	.upload-wrap {
		padding: 0.25rem 0.625rem 0.625rem;
	}

	.upload {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 0.4375rem;
		height: 36px;
		border-radius: var(--radius-md);
		background: var(--accent);
		color: #fff;
		font-size: var(--fs-sm);
		font-weight: var(--fw-medium);
		text-decoration: none;
		transition: background var(--dur-fast) var(--ease);

		&:hover {
			background: var(--accent-hover);
			color: #fff;
		}
	}

	.nav {
		display: flex;
		flex-direction: column;
		gap: 2px;
		padding-inline: var(--space-2);
		overflow-y: auto;
	}

	.item {
		display: flex;
		align-items: center;
		gap: 0.625rem;
		height: 34px;
		padding-inline: 0.625rem;
		border-radius: 8px;
		font-size: var(--fs-sm);
		font-weight: var(--fw-medium);
		color: var(--ink-mute);
		text-decoration: none;
		transition:
			background var(--dur-fast) var(--ease),
			color var(--dur-fast) var(--ease);

		&:hover {
			background: var(--nav-hover);
		}
		&.active {
			background: var(--accent-soft);
			color: var(--accent);
		}
		&.centered {
			justify-content: center;
		}
	}

	.divider {
		height: 1px;
		margin: var(--space-2) 0.625rem;
		background: var(--edge);
	}

	.foot {
		margin-top: auto;
		padding: 1rem 0.875rem;
		border-top: 1px solid var(--edge);
		display: flex;
		flex-direction: column;
		gap: 0.875rem;
	}

	.storage {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}

	.meter {
		height: 4px;
		border-radius: var(--radius-full);
		background: var(--tint-softer);
		overflow: hidden;
	}

	.fill {
		height: 100%;
		border-radius: var(--radius-full);
		background: var(--accent);
		transition: width var(--dur) var(--ease);

		&.warn {
			background: var(--warn);
		}
	}

	.storage-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-2);
	}

	.storage-text {
		font-family: var(--font-mono);
		font-size: var(--fs-xs);
		color: var(--ink-faint);
	}

	.get-more {
		font-size: var(--fs-xs);
		font-weight: var(--fw-medium);
		color: var(--ink-mute);
		text-decoration: none;
		white-space: nowrap;
		transition: color var(--dur-fast) var(--ease);

		&:hover {
			color: var(--ink);
		}
	}

	.user-row {
		display: flex;
		align-items: center;
		gap: var(--space-2);

		&.centered {
			justify-content: center;
		}
	}

	.details {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
	}

	.username {
		font-size: var(--fs-sm);
		font-weight: var(--fw-medium);
		color: var(--ink);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.email {
		font-size: var(--fs-xs);
		color: var(--ink-faint);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.logout-form {
		display: contents;
	}

	.nav-close {
		display: none;
		flex: 0 0 auto;
		width: 26px;
		height: 26px;
		border: 0;
		background: none;
		border-radius: var(--radius-sm);
		place-items: center;
		color: var(--ink-faint);
		cursor: pointer;
	}

	/* Mobile: off-canvas drawer, always full width, never collapsed. */
	@media (max-width: 768px) {
		.aside {
			position: fixed;
			inset-block: 0;
			left: 0;
			width: 260px;
			transform: translateX(-100%);
			transition: transform var(--dur) var(--ease);

			&.open {
				transform: none;
			}
			&.collapsed {
				width: 260px;
			}
		}
		.nav-close {
			display: grid;
		}
		.hide-mobile {
			display: none;
		}
	}
</style>
