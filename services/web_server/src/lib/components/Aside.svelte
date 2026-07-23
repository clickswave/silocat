<script>
	import { page } from '$app/stores';
	import SiloCatLogo from '$lib/assets/silo-cat.png';
	import Icon from '@iconify/svelte';
	import { FrontendClient } from '$lib/frontendClient.js';
	import { browser } from '$app/environment';
	import { createQuery } from '@tanstack/svelte-query';
	import { Progress, Avatar, ThemeToggle } from '$lib/ui';

	// `open` only matters on mobile, where the sidebar is an off-canvas drawer.
	let { open = false, onclose } = $props();

	const menuItems = [
		{ icon: 'ri:dashboard-line', label: 'Dashboard', href: '/home' },
		{ icon: 'ri:folder-line', label: 'Files', href: '/home/files' },
		{ icon: 'ri:share-line', label: 'Shared', href: '/home/shared' },
		{ icon: 'ri:star-line', label: 'Starred', href: '/home/starred' },
		{ icon: 'ri:delete-bin-line', label: 'Trash', href: '/home/trash' },
		{ icon: 'ri:bank-card-line', label: 'Billing', href: '/home/billing' },
		{ icon: 'ri:settings-3-line', label: 'Settings', href: '/home/settings' },
		{ icon: 'ri:customer-service-2-line', label: 'Support', href: '/home/support' }
	];

	let user = $derived($page.data.user || {});

	// Shared query key with the files page + dashboard: uploads/deletes there call
	// queryClient.invalidateQueries(['fetchStorageStats']), which refetches this and
	// keeps the sidebar meter live without a manual page refresh.
	const storageQuery = createQuery(() => ({
		queryKey: ['fetchStorageStats'],
		queryFn: async () => {
			// Trust the live stats endpoint: it sums base storage + every active,
			// non-expired subscription (promo / Pro grants). The session's
			// `totalAvailableSpace` only reflects a single subscription on the token
			// and goes stale, so it is a fallback only, never preferred.
			const { data } = await FrontendClient.get('/api/v1/sanctum/user/storage');
			return data?.success || { used: 0, total: 0 };
		},
		enabled: browser
	}));

	let storage = $derived({
		used: storageQuery.data?.used || 0,
		total: storageQuery.data?.total || user.totalAvailableSpace || 0
	});

	function formatSize(bytes) {
		if (!bytes) return '0 B';
		const k = 1024;
		const sizes = ['B', 'KB', 'MB', 'GB'];
		const i = Math.min(Math.floor(Math.log(bytes) / Math.log(k)), sizes.length - 1);
		return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
	}

	let usedPct = $derived(storage.total ? Math.min((storage.used / storage.total) * 100, 100) : 0);
</script>

<aside class="aside" class:open>
	<div class="header">
		<a href="/home" class="brand" title="Home">
			<img src={SiloCatLogo} alt="" />
			<span>silocat</span>
		</a>
		<button class="nav-close" onclick={onclose} aria-label="Close menu">
			<Icon icon="ri:close-line" width="20" />
		</button>
	</div>

	<nav>
		{#each menuItems as item (item.href)}
			<a href={item.href} class:active={$page.url.pathname === item.href}>
				<Icon icon={item.icon} width="17" class="nav-icon" />
				<span class="label">{item.label}</span>
			</a>
		{/each}
	</nav>

	<div class="foot">
		<div class="storage">
			<Progress value={usedPct} size="xs" tone={usedPct > 90 ? 'warn' : 'accent'} label="Storage used" />
			<span class="storage-text">{formatSize(storage.used)} / {formatSize(storage.total)}</span>
		</div>

		<div class="user-row">
			<Avatar src={user.profile_image} name={user.username} size={28} />
			<div class="details">
				<span class="username">{user.username || 'User'}</span>
				<span class="email" title={user.email}>{user.email || ''}</span>
			</div>
			<ThemeToggle />
			<form action="/auth/logout" method="POST" class="logout-form">
				<button type="submit" class="logout-btn" title="Sign out" aria-label="Sign out">
					<Icon icon="ri:logout-box-r-line" width="16" />
				</button>
			</form>
		</div>
	</div>
</aside>

<style lang="scss">
	.aside {
		width: 240px;
		flex-shrink: 0;
		background: var(--surface);
		border: 1px solid var(--edge);
		border-radius: var(--radius-md);
		display: flex;
		flex-direction: column;
		padding: var(--space-4);
		height: 100%;
		box-sizing: border-box;
		overflow-x: hidden;
		overflow-y: auto;
		z-index: 100;

		&::-webkit-scrollbar {
			width: 4px;
		}
		&::-webkit-scrollbar-thumb {
			background: var(--edge-strong);
			border-radius: var(--radius-full);
		}
	}

	.header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: var(--space-6);
		padding: var(--space-1) var(--space-2) 0;

		.brand {
			display: flex;
			align-items: center;
			gap: var(--space-2);
			color: var(--ink);

			img {
				width: 24px;
				height: 24px;
			}

			span {
				font-size: 1.05rem;
				font-weight: var(--fw-black);
				letter-spacing: var(--tracking-tight);
			}
		}

		.nav-close {
			display: none;
			align-items: center;
			justify-content: center;
			width: 30px;
			height: 30px;
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

	nav {
		display: flex;
		flex-direction: column;
		gap: 2px;
		flex: 1;

		a {
			position: relative;
			display: flex;
			align-items: center;
			gap: var(--space-3);
			height: 36px;
			padding: 0 var(--space-3);
			border-radius: var(--radius-sm);
			color: var(--ink-mute);
			font-size: var(--fs-sm);
			font-weight: var(--fw-medium);
			transition:
				color var(--dur) var(--ease),
				background var(--dur) var(--ease);

			:global(.nav-icon) {
				color: var(--ink-faint);
				flex-shrink: 0;
				transition: color var(--dur) var(--ease);
			}

			&:hover {
				color: var(--ink);
				background: var(--tint-soft);

				:global(.nav-icon) {
					color: var(--ink-mute);
				}
			}

			&.active {
				color: var(--ink);
				background: var(--accent-soft);

				:global(.nav-icon) {
					color: var(--accent);
				}

				&::before {
					content: '';
					position: absolute;
					left: 0;
					top: 50%;
					transform: translateY(-50%);
					height: 16px;
					width: 2px;
					background: var(--accent);
					border-radius: 0 2px 2px 0;
				}
			}
		}
	}

	.foot {
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
		padding-top: var(--space-4);
		border-top: 1px solid var(--edge);
	}

	.storage {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		padding: 0 var(--space-1);

		.storage-text {
			font-family: var(--font-mono);
			font-size: var(--fs-xs);
			color: var(--ink-faint);
		}
	}

	.user-row {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		min-width: 0;

		.details {
			flex: 1;
			display: flex;
			flex-direction: column;
			overflow: hidden;
			white-space: nowrap;
			min-width: 0;

			.username {
				font-size: var(--fs-sm);
				font-weight: var(--fw-medium);
				color: var(--ink);
				overflow: hidden;
				text-overflow: ellipsis;
			}

			.email {
				font-size: var(--fs-xs);
				color: var(--ink-faint);
				text-overflow: ellipsis;
				overflow: hidden;
			}
		}

		.logout-form {
			display: flex;
		}

		.logout-btn {
			display: flex;
			align-items: center;
			justify-content: center;
			width: 30px;
			height: 30px;
			background: none;
			border: none;
			border-radius: var(--radius-sm);
			color: var(--ink-mute);
			cursor: pointer;
			transition:
				color var(--dur) var(--ease),
				background var(--dur) var(--ease);

			&:hover {
				color: var(--danger);
				background: var(--danger-soft);
			}
		}
	}

	/* Mobile: off-canvas drawer toggled by the top-bar hamburger. */
	@media (max-width: 768px) {
		.aside {
			position: fixed;
			top: 0.75rem;
			left: 0.75rem;
			bottom: 0.75rem;
			height: auto;
			transform: translateX(calc(-100% - 1rem));
			transition: transform 0.25s cubic-bezier(0.2, 0, 0, 1);
			box-shadow: var(--shadow-overlay);
		}
		.aside.open {
			transform: none;
		}
		.aside .header .nav-close {
			display: flex;
		}
	}
</style>
