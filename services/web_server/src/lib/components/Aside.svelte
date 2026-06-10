<script>
	import { page } from '$app/stores';
	import SiloCatLogo from '$lib/assets/silo-cat.png';
	import Icon from '@iconify/svelte';
	import { FrontendClient } from '$lib/frontendClient.js';
	import { onMount } from 'svelte';
	import { fly } from 'svelte/transition';

	// `open` only matters on mobile, where the sidebar is an off-canvas drawer.
	let { open = false, onclose } = $props();

	const menuItems = [
		{ icon: 'ri:dashboard-line', label: 'Dashboard', href: '/home' },
		{ icon: 'ri:folder-line', label: 'My Files', href: '/home/files' },
		{ icon: 'ri:share-line', label: 'Shared', href: '/home/shared' },
		{ icon: 'ri:star-fill', label: 'Starred', href: '/home/starred' },
		{ icon: 'ri:delete-bin-line', label: 'Trash', href: '/home/trash' },
		{ icon: 'ri:bank-card-line', label: 'Billing', href: '/home/billing' },
		{ icon: 'ri:settings-3-line', label: 'Settings', href: '/home/settings' },
		{ icon: 'ri:customer-service-2-line', label: 'Support', href: '/home/support' }
	];

	let storage = $state({ used: 0, total: 0 });
	let user = $derived($page.data.user || {});

	onMount(async () => {
		try {
			// Fetch storage stats to get 'used' amount
			let { data } = await FrontendClient.get('/api/v1/sanctum/user/storage');
			if (data?.success) {
				// Trust the live stats endpoint: it sums base storage + every active,
				// non-expired subscription (promo / Pro grants). The session's
				// `totalAvailableSpace` only reflects a single subscription on the token
				// and goes stale (e.g. promo codes redeemed at signup are not on it), so
				// it is a fallback only — never preferred.
				storage = {
					used: data.success.used,
					total: data.success.total || user.totalAvailableSpace
				};
			}
		} catch (e) {
			console.error('Failed to fetch storage for sidebar', e);
			// Fallback
			if (user.totalAvailableSpace) {
				storage.total = user.totalAvailableSpace;
			}
		}
	});

	function formatSize(bytes) {
		if (!bytes) return '0 B';
		const k = 1024;
		const sizes = ['B', 'KB', 'MB', 'GB'];
		const i = Math.min(Math.floor(Math.log(bytes) / Math.log(k)), sizes.length - 1);
		return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
	}
</script>

<aside class="aside" class:open>
	<div class="header">
		<a href="/home" class="logo-container" title="Home">
			<img src={SiloCatLogo} alt="logo" class="logo-img" />
			<span class="logo-text">SILO.CAT</span>
		</a>
		<button class="nav-close" onclick={onclose} aria-label="Close menu">
			<Icon icon="ri:close-line" width="22" />
		</button>
	</div>

	<div class="divider"></div>

	<nav>
		{#each menuItems as item}
			<a href={item.href} class:active={$page.url.pathname === item.href}>
				<div class="icon-wrapper">
					<Icon icon={item.icon} width="22" class="nav-icon" />
				</div>
				<span class="label">{item.label}</span>
				{#if $page.url.pathname === item.href}
					<div class="active-indicator" transition:fly={{ x: -10, duration: 200 }}></div>
				{/if}
			</a>
		{/each}
	</nav>

	<div class="divider divider-foot"></div>

	<div class="user-section">
		<div class="user-info">
			<div class="avatar">
				{#if user.profile_image}
					<img src={user.profile_image} alt="User" referrerpolicy="no-referrer" />
				{:else}
					<Icon icon="ri:user-smile-line" width="24" color="#a1a1aa" />
				{/if}
			</div>
			<div class="details">
				<span class="username">{user.username || 'User'}</span>
				<span class="email" title={user.email}>{user.email || ''}</span>
			</div>
			<form action="/auth/logout" method="POST" class="logout-form">
				<button type="submit" class="logout-btn" title="Logout">
					<Icon icon="ri:logout-box-r-line" width="18" />
				</button>
			</form>
		</div>
		<div class="storage-bar-container">
			<div class="storage-info">
				<span>{formatSize(storage.used)} / {formatSize(storage.total)}</span>
			</div>
			<div class="progress-bar">
				<div
					class="fill"
					style="width: {storage.total
						? Math.min((storage.used / storage.total) * 100, 100)
						: 0}%"
				></div>
			</div>
		</div>
	</div>
</aside>

<style lang="scss">
	.aside {
		width: 280px;
		flex-shrink: 0;
		background: var(--bg-sidebar-glass);
		backdrop-filter: blur(20px);
		-webkit-backdrop-filter: blur(20px);
		display: flex;
		flex-direction: column;
		padding: var(--space-5) var(--space-4);
		border-radius: var(--radius-lg);
		height: 100%;
		box-sizing: border-box;
		border: 1px solid var(--border-sidebar);
		overflow-x: hidden;
		overflow-y: auto;
		z-index: 100;
		color: var(--text-primary);

		/* Scrollbar Styling */
		&::-webkit-scrollbar {
			width: 4px;
		}
		&::-webkit-scrollbar-thumb {
			background: var(--border-strong);
			border-radius: var(--radius-pill);
		}

		.header {
			display: flex;
			flex-direction: row;
			align-items: center;
			justify-content: center;
			gap: var(--space-2);
			margin-bottom: var(--space-6);
			min-height: 40px;
			padding: 0 var(--space-1);

			.logo-container {
				display: flex;
				align-items: center;
				gap: 10px;
				overflow: hidden;
				text-decoration: none;
				color: inherit;
				border-radius: var(--radius-sm);

				.logo-img {
					width: 36px;
					height: 36px;
					flex-shrink: 0;
					filter: drop-shadow(0 0 10px rgba(255, 255, 255, 0.1));
				}

				.logo-text {
					font-weight: var(--fw-black);
					/* match the wordmark height to the 36px logo */
					font-size: 1.7rem;
					line-height: 36px;
					letter-spacing: 0.02em;
					white-space: nowrap;
				}

				&:hover .logo-text {
					color: var(--primary);
				}
			}

			/* Close button only shows in the mobile drawer. */
			.nav-close {
				display: none;
				width: 36px;
				height: 36px;
				flex-shrink: 0;
				align-items: center;
				justify-content: center;
				background: var(--bg-card);
				border: 1px solid var(--border-sidebar);
				border-radius: var(--radius-md);
				color: var(--text-muted);
				cursor: pointer;

				&:hover {
					background: var(--nav-hover);
					color: var(--text-primary);
				}
			}
		}

		.divider {
			height: 1px;
			background: var(--border-sidebar);
			margin: 0 calc(-1 * var(--space-4)) var(--space-5) calc(-1 * var(--space-4));
		}

		/* Same full-bleed rule between body and footer, with space above it too. */
		.divider-foot {
			margin-top: var(--space-5);
		}

		nav {
			display: flex;
			flex-direction: column;
			gap: var(--space-2);
			flex: 1;

			a {
				text-decoration: none;
				color: var(--text-muted);
				position: relative;
				display: flex;
				align-items: center;
				height: 48px;
				border-radius: var(--radius-md);
				transition: color var(--dur) var(--ease), background var(--dur) var(--ease);
				padding: 0 var(--space-3);
				overflow: hidden;

				.icon-wrapper {
					display: flex;
					align-items: center;
					justify-content: center;
					width: 24px;
					flex-shrink: 0;
				}

				.label {
					margin-left: var(--space-3);
					font-weight: var(--fw-medium);
					font-size: var(--fs-sm);
					white-space: nowrap;
				}

				&:hover {
					color: var(--text-primary);
					background: var(--nav-hover);
				}

				&.active {
					color: var(--text-primary); /* Ensure contrast */
					background: linear-gradient(
						90deg,
						rgba(255, 70, 85, 0.15) 0%,
						rgba(255, 70, 85, 0.05) 100%
					);
					border: 1px solid rgba(255, 70, 85, 0.18);

					:global(.nav-icon) {
						color: var(--primary);
					}

					.active-indicator {
						position: absolute;
						left: 0;
						height: 20px;
						width: 3px;
						background: var(--primary);
						border-radius: 0 var(--radius-sm) var(--radius-sm) 0;
					}
				}
			}
		}

		.user-section {
			/* Separation comes from .divider-foot above, full-bleed like the header. */
			.user-info {
				display: flex;
				align-items: center;
				gap: var(--space-3);
				min-height: 44px;

				.avatar {
					width: 40px;
					height: 40px;
					border-radius: var(--radius-pill);
					background: var(--tint-soft);
					display: flex;
					align-items: center;
					justify-content: center;
					flex-shrink: 0;
					border: 1px solid var(--border-default);
					overflow: hidden;

					img {
						width: 100%;
						height: 100%;
						object-fit: cover;
					}
				}

				.details {
					flex: 1;
					display: flex;
					flex-direction: column;
					overflow: hidden;
					white-space: nowrap;

					.username {
						font-weight: var(--fw-semibold);
						font-size: var(--fs-sm);
						color: var(--text-primary);
					}

					.email {
						font-size: var(--fs-xs);
						color: var(--text-muted);
						text-overflow: ellipsis;
						overflow: hidden;
					}
				}

				.logout-form {
					margin-left: auto;
				}

				.logout-btn {
					background: transparent;
					border: none;
					color: var(--text-muted);
					cursor: pointer;
					padding: var(--space-2);
					border-radius: var(--radius-sm);
					display: flex;
					transition: color var(--dur) var(--ease), background var(--dur) var(--ease);

					&:hover {
						color: var(--primary);
						background: rgba(255, 70, 85, 0.1);
					}
				}
			}

			.storage-bar-container {
				margin-top: var(--space-4);
				background: var(--tint-soft);
				border-radius: var(--radius-md);
				padding: var(--space-3);
				border: 1px solid var(--border-default);

				.storage-info {
					display: flex;
					justify-content: space-between;
					font-size: var(--fs-xs);
					color: var(--text-muted);
					margin-bottom: var(--space-2);
					font-weight: var(--fw-medium);
				}

				.progress-bar {
					height: 6px;
					background: var(--tint-softer);
					border-radius: var(--radius-pill);
					overflow: hidden;

					.fill {
						height: 100%;
						background: var(--accent-gradient);
						border-radius: var(--radius-pill);
						transition: width 0.5s ease-out;
					}
				}
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
			box-shadow: 0 20px 60px rgba(0, 0, 0, 0.5);
		}
		.aside.open {
			transform: none;
		}
		.aside .header {
			justify-content: space-between;
		}
		.aside .header .nav-close {
			display: flex;
		}
	}
</style>
