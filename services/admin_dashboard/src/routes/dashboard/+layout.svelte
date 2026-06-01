<script>
	import Icon from '@iconify/svelte';
	import { page } from '$app/stores';
	import { QueryClient, QueryClientProvider } from '@tanstack/svelte-query';

	let { children, data } = $props();

	const queryClient = new QueryClient({
		defaultOptions: {
			queries: {
				enabled: typeof window !== 'undefined'
			}
		}
	});

	let activeRoute = $derived($page.url.pathname);
</script>

<QueryClientProvider client={queryClient}>
	<div class="dashboard-container">
		<aside class="sidebar">
			<div class="logo">
				<h2>Silo<span>Cat</span></h2>
			</div>

			<nav>
				<a href="/dashboard" class="nav-item" class:active={activeRoute === '/dashboard'}>
					<Icon icon="ri:dashboard-line" width="20" />
					<span>Overview</span>
				</a>
				<a
					href="/dashboard/files"
					class="nav-item"
					class:active={activeRoute === '/dashboard/files'}
				>
					<Icon icon="ri:file-list-3-line" width="20" />
					<span>Files</span>
				</a>
				<a
					href="/dashboard/invites"
					class="nav-item"
					class:active={activeRoute === '/dashboard/invites'}
				>
					<Icon icon="ri:mail-send-line" width="20" />
					<span>Invite Codes</span>
				</a>
				<a
					href="/dashboard/promos"
					class="nav-item"
					class:active={activeRoute === '/dashboard/promos'}
				>
					<Icon icon="ri:ticket-2-line" width="20" />
					<span>Promos</span>
				</a>
				<a
					href="/dashboard/users"
					class="nav-item"
					class:active={activeRoute === '/dashboard/users'}
				>
					<Icon icon="ri:user-line" width="20" />
					<span>Users</span>
				</a>
				<a
					href="/dashboard/anon-users"
					class="nav-item"
					class:active={activeRoute === '/dashboard/anon-users'}
				>
					<Icon icon="ri:spy-line" width="20" />
					<span>Anon Users</span>
				</a>
				<a
					href="/dashboard/orders"
					class="nav-item"
					class:active={activeRoute === '/dashboard/orders'}
				>
					<Icon icon="ri:shopping-cart-2-line" width="20" />
					<span>Orders</span>
				</a>
				<a
					href="/dashboard/subscriptions"
					class="nav-item"
					class:active={activeRoute === '/dashboard/subscriptions'}
				>
					<Icon icon="ri:vip-crown-line" width="20" />
					<span>Subscriptions</span>
				</a>
				<a
					href="/dashboard/early-access"
					class="nav-item"
					class:active={activeRoute === '/dashboard/early-access'}
				>
					<Icon icon="ri:rocket-line" width="20" />
					<span>Early Access</span>
				</a>

				<div style="flex: 1"></div>

				<form action="/logout" method="POST">
					<button type="submit" class="nav-item logout">
						<Icon icon="ri:logout-box-line" width="20" />
						<span>Logout</span>
					</button>
				</form>
			</nav>
		</aside>

		<main class="main-content">
			{@render children()}
		</main>
	</div>
</QueryClientProvider>

<style lang="scss">
	:global(body) {
		background: var(--bg-app);
		/* Subtle red background glow */
		background-image: radial-gradient(circle at 50% 0%, rgba(255, 26, 26, 0.05), transparent 60%);
		background-attachment: fixed;
	}

	.dashboard-container {
		display: flex;
		height: 100vh;
		width: 100vw;
		overflow: hidden;
		background: var(--bg-app);
	}

	.sidebar {
		width: 260px;
		background: var(--bg-sidebar);
		border-right: 1px solid var(--border-default);
		display: flex;
		flex-direction: column;
		flex-shrink: 0;
		padding: 1.5rem;
		z-index: 20;

		.logo {
			margin-bottom: 2.5rem;
			padding-left: 0.5rem;

			h2 {
				font-size: 1.5rem;
				font-weight: 800;
				margin: 0;
				color: var(--text-primary);
				letter-spacing: -0.05em;

				span {
					color: var(--primary);
				}
			}
		}

		nav {
			display: flex;
			flex-direction: column;
			gap: 0.5rem;
			flex: 1;
			overflow-y: auto;
			padding-right: 0.5rem;

			/* Thin scrollbar for sidebar nav if needed */
			&::-webkit-scrollbar {
				width: 4px;
			}
			&::-webkit-scrollbar-thumb {
				background: var(--border-default);
				border-radius: 4px;
			}

			.nav-item {
				display: flex;
				align-items: center;
				gap: 0.8rem;
				padding: 0.8rem 1rem;
				border-radius: var(--radius-md);
				color: var(--text-secondary);
				text-decoration: none;
				transition: all 0.2s cubic-bezier(0.16, 1, 0.3, 1);
				font-weight: 500;
				border: 1px solid transparent;
				white-space: nowrap;

				&:hover {
					background: var(--bg-card-hover);
					color: var(--text-primary);
				}

				&.active {
					background: var(--nav-hover);
					color: var(--primary);
					border-color: rgba(255, 26, 26, 0.1);
					box-shadow: 0 0 20px rgba(255, 26, 26, 0.1);
				}

				&.logout {
					background: transparent;
					border: none;
					cursor: pointer;
					width: 100%;
					margin-top: auto;
					font-size: 1rem;

					&:hover {
						background: rgba(255, 26, 26, 0.1);
						color: var(--danger);
						border-color: rgba(255, 26, 26, 0.1);
					}
				}
			}
		}
	}

	.main-content {
		flex: 1;
		height: 100vh;
		overflow-y: auto;
		overflow-x: hidden;
		padding: 2rem;
		position: relative;
		scroll-behavior: smooth;
	}
</style>
