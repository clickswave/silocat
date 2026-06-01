<script>
	import { page } from '$app/stores';
	import SiloCatLogo from '$lib/assets/silo-cat.png';
	import Icon from '@iconify/svelte';
	import { FrontendClient } from '$lib/frontendClient.js';
	import { onMount } from 'svelte';
	import { fly, fade } from 'svelte/transition';

	const menuItems = [
		{ icon: 'ri:dashboard-line', label: 'Dashboard', href: '/home' },
		{ icon: 'ri:folder-line', label: 'My Files', href: '/home/files' },
		{ icon: 'ri:share-line', label: 'Shared', href: '/home/shared' },
		{ icon: 'ri:star-fill', label: 'Starred', href: '/home/starred' },
		{ icon: 'ri:delete-bin-line', label: 'Trash', href: '/home/trash' },
		{ icon: 'ri:bank-card-line', label: 'Billing', href: '/home/billing' },
		{ icon: 'ri:settings-3-line', label: 'Settings', href: '/home/settings' }
	];

	let storage = $state({ used: 0, total: 0 });
	let user = $derived($page.data.user || {});
	let isExpanded = $state(true); // Default to expanded

	onMount(async () => {
		try {
			// Fetch storage stats to get 'used' amount
			let { data } = await FrontendClient.get('/api/v1/sanctum/user/storage');
			if (data?.success) {
				storage = {
					used: data.success.used,
					total: user.totalAvailableSpace || data.success.total
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

	function toggleSidebar() {
		isExpanded = !isExpanded;
	}
</script>

<aside class="aside" class:expanded={isExpanded}>
	<div class="header">
		<div class="logo-container">
			<img src={SiloCatLogo} alt="logo" class="logo-img" />
			{#if isExpanded}
				<span class="logo-text" transition:fade={{ duration: 200 }}>SILO.CAT</span>
			{/if}
		</div>
	</div>

	<div class="divider"></div>

	<nav>
		{#each menuItems as item}
			<a
				href={item.href}
				class:active={$page.url.pathname === item.href}
				title={!isExpanded ? item.label : ''}
			>
				<div class="icon-wrapper">
					<Icon icon={item.icon} width="22" class="nav-icon" />
				</div>
				{#if isExpanded}
					<span class="label" transition:fade={{ duration: 150 }}>{item.label}</span>
				{/if}
				{#if $page.url.pathname === item.href}
					<div class="active-indicator" transition:fly={{ x: -10, duration: 200 }}></div>
				{/if}
			</a>
		{/each}
	</nav>

	<div class="footer-action">
		<button
			class="sidebar-toggle-btn"
			class:compact={!isExpanded}
			onclick={toggleSidebar}
			title={isExpanded ? 'Collapse Menu' : 'Expand Menu'}
		>
			<Icon icon={isExpanded ? 'ri:menu-fold-line' : 'ri:menu-unfold-line'} width="24" />
			{#if isExpanded}
				<span class="label" transition:fade={{ duration: 150 }}>Collapse Menu</span>
			{/if}
		</button>
	</div>

	<div class="user-section">
		<div class="user-info">
			<div class="avatar">
				{#if user.avatar_url}
					<img src={user.avatar_url} alt="User" />
				{:else}
					<Icon icon="ri:user-smile-line" width="24" color="#a1a1aa" />
				{/if}
			</div>
			{#if isExpanded}
				<div class="details" transition:fade={{ duration: 150 }}>
					<span class="username">{user.username || 'User'}</span>
					<span class="email" title={user.email}>{user.email || ''}</span>
				</div>
				<form action="/auth/logout" method="POST" class="logout-form">
					<button type="submit" class="logout-btn" title="Logout">
						<Icon icon="ri:logout-box-r-line" width="18" />
					</button>
				</form>
			{/if}
		</div>
		{#if isExpanded}
			<div class="storage-bar-container" transition:fade={{ duration: 150 }}>
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
		{/if}
	</div>
</aside>

<style lang="scss">
	.aside {
		width: 80px; /* Collapsed width */
		background: var(--bg-sidebar-glass);
		backdrop-filter: blur(20px);
		-webkit-backdrop-filter: blur(20px);
		display: flex;
		flex-direction: column;
		padding: 1.5rem 1rem;
		border-radius: 20px;
		height: calc(100vh - 40px);
		box-sizing: border-box;
		margin: 20px 0 20px 20px;
		border: 1px solid var(--border-sidebar);
		transition: width 0.3s cubic-bezier(0.2, 0, 0, 1);
		overflow-x: hidden;
		overflow-y: auto;
		z-index: 100;
		color: var(--text-primary);

		&.expanded {
			width: 280px;
		}

		/* Scrollbar Styling */
		&::-webkit-scrollbar {
			width: 4px;
		}
		&::-webkit-scrollbar-thumb {
			background: rgba(255, 255, 255, 0.1);
			border-radius: 2px;
		}

		.header {
			display: flex;
			align-items: center;
			justify-content: space-between;
			margin-bottom: 2rem;
			padding: 0 4px;
			min-height: 40px;

			.logo-container {
				display: flex;
				align-items: center;
				gap: 12px;
				overflow: hidden;

				.logo-img {
					width: 36px;
					height: 36px;
					flex-shrink: 0;
					filter: drop-shadow(0 0 10px rgba(255, 255, 255, 0.1));
				}

				.logo-text {
					font-weight: 700;
					font-size: 1.25rem;
					letter-spacing: -0.02em;
					white-space: nowrap;
				}
			}

			/* toggle-btn removed */
		}

		.divider {
			height: 1px;
			background: var(--border-sidebar);
			margin: 0 -1rem 1.5rem -1rem;
		}

		nav {
			display: flex;
			flex-direction: column;
			gap: 0.5rem;
			flex: 1;

			a {
				text-decoration: none;
				color: var(--text-muted);
				position: relative;
				display: flex;
				align-items: center;
				height: 48px;
				border-radius: 12px;
				transition: all 0.2s;
				padding: 0 12px;
				overflow: hidden;

				.icon-wrapper {
					display: flex;
					align-items: center;
					justify-content: center;
					width: 24px;
					flex-shrink: 0;
				}

				.label {
					margin-left: 12px;
					font-weight: 500;
					font-size: 0.95rem;
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
					border: 1px solid rgba(255, 70, 85, 0.1);

					:global(.nav-icon) {
						color: var(--primary);
					}

					.active-indicator {
						position: absolute;
						left: 0;
						height: 20px;
						width: 3px;
						background: var(--primary);
						border-radius: 0 4px 4px 0;
					}
				}
			}
		}

		.footer-action {
			margin-top: 2rem;

			.sidebar-toggle-btn {
				width: 100%;
				height: 52px;
				border-radius: 14px;
				background: var(--bg-card); /* Neutral background */
				border: 1px solid var(--border-sidebar);
				color: var(--text-muted);
				cursor: pointer;
				display: flex;
				align-items: center;
				justify-content: center;
				gap: 12px;
				font-weight: 500;
				font-size: 0.95rem;
				transition: all 0.2s;
				overflow: hidden;

				&.compact {
					width: 48px;
					margin: 0 auto;
					padding: 0;
				}

				&:hover {
					background: var(--nav-hover);
					color: var(--text-primary);
					transform: translateY(-1px);
				}

				&:active {
					transform: translateY(0);
				}
			}
		}

		.user-section {
			margin-top: 2rem;
			padding-top: 1.5rem;
			border-top: 1px solid rgba(255, 255, 255, 0.1);

			.user-info {
				display: flex;
				align-items: center;
				gap: 12px;
				min-height: 44px;

				.avatar {
					width: 40px;
					height: 40px;
					border-radius: 50%;
					background: rgba(255, 255, 255, 0.05);
					display: flex;
					align-items: center;
					justify-content: center;
					flex-shrink: 0;
					border: 1px solid rgba(255, 255, 255, 0.1);
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
						font-weight: 600;
						font-size: 0.95rem;
						color: white;
					}

					.email {
						font-size: 0.8rem;
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
					padding: 8px;
					border-radius: 8px;
					transition: all 0.2s;

					&:hover {
						color: var(--primary);
						background: rgba(255, 70, 85, 0.1);
					}
				}
			}

			.storage-bar-container {
				margin-top: 1rem;
				background: rgba(0, 0, 0, 0.2);
				border-radius: 12px;
				padding: 12px;
				border: 1px solid rgba(255, 255, 255, 0.05);

				.storage-info {
					display: flex;
					justify-content: space-between;
					font-size: 0.8rem;
					color: var(--text-muted);
					margin-bottom: 8px;
					font-weight: 500;
				}

				.progress-bar {
					height: 6px;
					background: rgba(255, 255, 255, 0.1);
					border-radius: 3px;
					overflow: hidden;

					.fill {
						height: 100%;
						background: var(--primary, #ff4655);
						border-radius: 3px;
						transition: width 0.5s ease-out;
					}
				}
			}
		}
	}
</style>
