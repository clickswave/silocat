<script>
	import SiloCatLogo from '$lib/assets/silo-cat.png';
	import { page } from '$app/stores';
	import Icon from '@iconify/svelte';
	import { shadowKey, regenerateShadowKey, clearShadowKey } from '$lib/stores/shadow.js';
	import { onMount } from 'svelte';
	import { fade, scale } from 'svelte/transition';

	let showShadowModal = $state(false);

	let isMobileMenuOpen = $state(false);

	function toggleMobileMenu() {
		isMobileMenuOpen = !isMobileMenuOpen;
	}

	function closeMobileMenu() {
		isMobileMenuOpen = false;
	}

	function toggleShadowModal() {
		showShadowModal = !showShadowModal;
	}

	function handleRegenerate() {
		regenerateShadowKey();
	}

	function handleClear() {
		clearShadowKey();
		showShadowModal = false;
	}
</script>

<nav class="landing-nav">
	<div class="left-section">
		<a href="/" class="logo" style="text-decoration: none; color: inherit;">
			<img src={SiloCatLogo} alt="SiloCat" />
			<span>SILO.CAT</span>
		</a>

		{#if $page.data.user}
			<button
				class="shadow-badge"
				onclick={toggleShadowModal}
				style="border-color: rgba(255, 70, 85, 0.4); color: white;"
			>
				<Icon icon="ri:shield-user-fill" style="color: #ff4655;" />
				<span>Account Key Active</span>
			</button>
		{:else if $shadowKey}
			<button class="shadow-badge" onclick={toggleShadowModal}>
				<Icon icon="ri:spy-line" />
				<span>Browser Key Active</span>
			</button>
		{/if}
	</div>

	<!-- Desktop Nav Links -->
	<div class="nav-links desktop-only">
		<a href="/early-access" class="nav-link" class:active={$page.url.pathname === '/early-access'}>
			<Icon icon="ri:flashlight-line" width="18" />
			<span>Early Access</span>
		</a>
		<a href="/api" class="nav-link" class:active={$page.url.pathname === '/api'}>
			<Icon icon="ri:code-s-slash-line" width="18" />
			<span>APIs</span>
		</a>
		<a href="/pricing" class="nav-link" class:active={$page.url.pathname === '/pricing'}>
			<Icon icon="ri:price-tag-3-line" width="18" />
			<span>Pricing</span>
		</a>
		<a href="/about" class="nav-link" class:active={$page.url.pathname === '/about'}>
			<Icon icon="ri:information-line" width="18" />
			<span>About</span>
		</a>
		<a href="/privacy" class="nav-link" class:active={$page.url.pathname === '/privacy'}>
			<Icon icon="ri:shield-keyhole-line" width="18" />
			<span>Privacy</span>
		</a>
		<div class="divider"></div>
		<a href="/auth/signin" class="nav-link">
			<Icon icon="ri:login-box-line" width="18" />
			<span>Sign In</span>
		</a>
		<a href="/auth/signup" class="btn-primary">
			<Icon icon="ri:user-add-line" width="18" />
			<span>Sign Up</span>
		</a>
	</div>

	<!-- Mobile Menu Button -->
	<button class="mobile-menu-btn" onclick={toggleMobileMenu} aria-label="Menu">
		<Icon icon="ri:menu-4-line" width="24" />
	</button>
</nav>

<!-- Mobile Menu Modal -->
{#if isMobileMenuOpen}
	<div class="mobile-menu-backdrop" transition:fade onclick={closeMobileMenu}>
		<div
			class="mobile-menu-content"
			transition:scale={{ start: 0.9, duration: 200 }}
			onclick={(e) => e.stopPropagation()}
		>
			<div class="mobile-header">
				<span class="menu-title">Menu</span>
				<button class="close-menu-btn" onclick={closeMobileMenu}>
					<Icon icon="ri:close-line" width="24" />
				</button>
			</div>

			<div class="mobile-links">
				{#if $page.data.user}
					<button
						class="mobile-shadow-badge"
						onclick={() => {
							closeMobileMenu();
							toggleShadowModal();
						}}
						style="border-color: rgba(255, 70, 85, 0.4); color: white;"
					>
						<Icon icon="ri:shield-user-fill" style="color: #ff4655;" width="20" />
						<span>Account Key Active</span>
					</button>
				{:else if $shadowKey}
					<button
						class="mobile-shadow-badge"
						onclick={() => {
							closeMobileMenu();
							toggleShadowModal();
						}}
					>
						<Icon icon="ri:spy-line" width="20" />
						<span>Browser Key Active</span>
					</button>
				{/if}

				<a href="/early-access" class="mobile-link" onclick={closeMobileMenu}>
					<Icon icon="ri:flashlight-line" width="20" />
					<span>Early Access</span>
				</a>
				<a href="/api" class="mobile-link" onclick={closeMobileMenu}>
					<Icon icon="ri:code-s-slash-line" width="20" />
					<span>APIs</span>
				</a>
				<a href="/pricing" class="mobile-link" onclick={closeMobileMenu}>
					<Icon icon="ri:price-tag-3-line" width="20" />
					<span>Pricing</span>
				</a>
				<a href="/about" class="mobile-link" onclick={closeMobileMenu}>
					<Icon icon="ri:information-line" width="20" />
					<span>About</span>
				</a>
				<a href="/privacy" class="mobile-link" onclick={closeMobileMenu}>
					<Icon icon="ri:shield-keyhole-line" width="20" />
					<span>Privacy</span>
				</a>

				<div class="mobile-divider"></div>

				<a href="/auth/signin" class="mobile-link" onclick={closeMobileMenu}>
					<Icon icon="ri:login-box-line" width="20" />
					<span>Sign In</span>
				</a>
				<a href="/auth/signup" class="mobile-btn-primary" onclick={closeMobileMenu}>
					<Icon icon="ri:user-add-line" width="20" />
					<span>Sign Up</span>
				</a>
			</div>
		</div>
	</div>
{/if}

{#if showShadowModal}
	<div class="modal-backdrop" transition:fade onclick={toggleShadowModal}>
		<!-- stopPropagation prevents closing when clicking content -->
		<div class="modal-content" transition:scale onclick={(e) => e.stopPropagation()}>
			<div class="modal-header">
				{#if $page.data.user}
					<Icon icon="ri:shield-user-fill" width="32" class="modal-icon" />
					<h2>Identity</h2>
				{:else}
					<Icon icon="ri:spy-fill" width="32" class="modal-icon" />
					<h2>Shadow Identity</h2>
				{/if}
			</div>

			<p class="modal-desc">
				{#if $page.data.user}
					You are currently <strong>Logged In</strong>. We are using your account's
					<strong>API Key</strong> for all uploads instead of the browser's shadow key.
				{:else}
					This browser has a unique <strong>API Key</strong> saved locally. It allows you to manage "anonymous"
					uploads across sessions without an account.
				{/if}
			</p>

			{#if $page.data.user || $shadowKey}
				<div class="key-display">
					<code>{$page.data.user ? $page.data.user.api_key : $shadowKey}</code>
					<button
						class="copy-btn"
						onclick={() =>
							navigator.clipboard.writeText($page.data.user ? $page.data.user.api_key : $shadowKey)}
					>
						<Icon icon="ri:file-copy-line" />
					</button>
				</div>
			{/if}

			{#if !$page.data.user}
				<div class="modal-actions">
					<button class="action-btn regenerate" onclick={handleRegenerate}>
						<Icon icon="ri:refresh-line" /> Regenerate
					</button>
					<button class="action-btn delete" onclick={handleClear}>
						<Icon icon="ri:delete-bin-line" /> Delete Key
					</button>
				</div>
			{/if}

			<button class="close-btn" onclick={toggleShadowModal}>Close</button>
		</div>
	</div>
{/if}

<style lang="scss">
	.landing-nav {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 1.25rem 2rem;
		z-index: 50;
		background: var(--bg-sidebar-glass);
		backdrop-filter: blur(16px);
		border-bottom: 1px solid var(--border-default);
		position: sticky;
		top: 0;
		transition: all 0.3s ease;

		.left-section {
			display: flex;
			align-items: center;
			gap: 1.5rem;

			.logo {
				display: flex;
				align-items: center;
				gap: 0.75rem;
				font-weight: 800;
				font-size: 1.4rem;
				letter-spacing: 0.05em;
				transition: transform 0.2s;

				&:hover {
					transform: scale(1.02);
				}

				img {
					width: 36px;
					height: 36px;
					filter: drop-shadow(0 0 8px rgba(255, 255, 255, 0.2));
				}

				span {
					background: var(--text-primary);
					-webkit-background-clip: text;
					-webkit-text-fill-color: transparent;
				}
			}

			.shadow-badge {
				background: var(--bg-card);
				border: 1px solid var(--border-default);
				padding: 0.4rem 0.8rem;
				border-radius: 20px;
				color: var(--text-muted);
				font-size: 0.85rem;
				font-weight: 500;
				display: flex;
				align-items: center;
				gap: 0.5rem;
				cursor: pointer;
				transition: all 0.2s;

				&:hover {
					background: var(--bg-card-hover);
					color: var(--text-primary);
					border-color: var(--border-active);
				}
			}
		}

		.nav-links {
			display: flex;
			align-items: center;
			gap: 2rem;
			/* ... existing nav-links styles ... */

			.nav-link {
				color: var(--text-muted);
				text-decoration: none;
				font-weight: 500;
				font-size: 0.95rem;
				display: flex;
				align-items: center;
				gap: 0.5rem;
				transition: all 0.2s ease;
				position: relative;
				padding: 0.5rem 0;

				&:hover,
				&.active {
					color: var(--text-primary);
					text-shadow: none;
				}

				&::after {
					content: '';
					position: absolute;
					bottom: 0px;
					left: 0;
					width: 0;
					height: 2px;
					background: var(--primary, #ff4655);
					transition: width 0.3s ease;
					border-radius: 2px;
				}

				&:hover::after,
				&.active::after {
					width: 100%;
				}
			}

			.divider {
				width: 1px;
				height: 24px;
				background-color: var(--border-default);
				margin: 0 0.5rem;
			}

			.btn-primary {
				background: var(--primary, #ff4655);
				color: white;
				text-decoration: none;
				padding: 0.7rem 1.4rem;
				border-radius: 12px;
				font-weight: 600;
				transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
				font-size: 0.95rem;
				display: flex;
				align-items: center;
				gap: 0.5rem;
				border: 1px solid rgba(255, 255, 255, 0.1);

				&:hover {
					background: #e03e4b;
					transform: translateY(-2px);
					box-shadow: 0 4px 20px rgba(255, 70, 85, 0.4);
					border-color: rgba(255, 255, 255, 0.2);
				}

				&:active {
					transform: translateY(0);
				}
			}
		}
	}

	.modal-backdrop {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.7);
		backdrop-filter: blur(8px);
		z-index: 100;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 1rem;
	}

	.modal-content {
		/* ... existing modal styles ... */
		background: var(--bg-card);
		border: 1px solid var(--border-default);
		border-radius: 20px;
		padding: 2rem;
		width: 100%;
		max-width: 480px;
		display: flex;
		flex-direction: column;
		gap: 1.5rem;
		box-shadow: var(--shadow-card);
		position: relative;

		.modal-header {
			display: flex;
			align-items: center;
			gap: 1rem;

			:global(.modal-icon) {
				color: var(--primary, #ff4655);
			}

			h2 {
				margin: 0;
				font-size: 1.5rem;
				color: var(--text-primary);
			}
		}

		.modal-desc {
			margin: 0;
			color: var(--text-muted);
			line-height: 1.5;
			font-size: 0.95rem;
		}

		.key-display {
			background: var(--bg-input);
			border: 1px solid var(--border-default);
			border-radius: 12px;
			padding: 1rem;
			display: flex;
			align-items: center;
			justify-content: space-between;
			gap: 1rem;

			code {
				color: var(--primary, #ff4655);
				font-family: monospace;
				font-size: 1.1rem;
				overflow: hidden;
				text-overflow: ellipsis;
			}

			.copy-btn {
				background: transparent;
				border: none;
				color: var(--text-muted);
				cursor: pointer;
				padding: 0.5rem;
				border-radius: 8px;
				transition: all 0.2s;

				&:hover {
					background: var(--nav-hover);
					color: var(--text-primary);
				}
			}
		}

		.modal-actions {
			display: grid;
			grid-template-columns: 1fr 1fr;
			gap: 1rem;

			.action-btn {
				padding: 0.875rem;
				border-radius: 12px;
				font-weight: 500;
				display: flex;
				align-items: center;
				justify-content: center;
				gap: 0.5rem;
				cursor: pointer;
				transition: all 0.2s;
				font-size: 0.95rem;

				&.regenerate {
					background: var(--bg-input);
					border: 1px solid var(--border-default);
					color: var(--text-primary);

					&:hover {
						background: var(--bg-card-hover);
					}
				}

				&.delete {
					background: rgba(239, 68, 68, 0.1);
					border: 1px solid rgba(239, 68, 68, 0.2);
					color: #ef4444;

					&:hover {
						background: rgba(239, 68, 68, 0.2);
						border-color: rgba(239, 68, 68, 0.3);
					}
				}
			}
		}

		.close-btn {
			align-self: flex-end;
			background: transparent;
			border: none;
			color: var(--text-muted);
			font-size: 0.9rem;
			cursor: pointer;
			text-decoration: underline;

			&:hover {
				color: var(--text-primary);
			}
		}
	}

	/* Mobile Menu Styles */
	.mobile-menu-btn {
		display: none;
		background: transparent;
		border: none;
		color: var(--text-muted);
		cursor: pointer;
		padding: 0.5rem;
		border-radius: 8px;

		&:hover {
			color: var(--text-primary);
			background: var(--bg-input);
		}
	}

	.mobile-menu-backdrop {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.7);
		backdrop-filter: blur(8px);
		z-index: 100;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 1rem;
	}

	.mobile-menu-content {
		background: var(--bg-card);
		border: 1px solid var(--border-default);
		border-radius: 20px;
		padding: 1.5rem;
		width: 100%;
		max-width: 320px;
		display: flex;
		flex-direction: column;
		gap: 1.5rem;
		box-shadow: var(--shadow-card);
	}

	.mobile-header {
		display: flex;
		justify-content: space-between;
		align-items: center;

		.menu-title {
			font-size: 1.25rem;
			font-weight: 700;
			color: var(--text-primary);
		}

		.close-menu-btn {
			background: transparent;
			border: none;
			color: var(--text-muted);
			cursor: pointer;
			padding: 0.25rem;

			&:hover {
				color: var(--text-primary);
			}
		}
	}

	.mobile-links {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;

		.mobile-link {
			text-decoration: none;
			color: var(--text-muted);
			font-size: 1rem;
			font-weight: 500;
			padding: 0.75rem 1rem;
			border-radius: 12px;
			display: flex;
			align-items: center;
			gap: 0.75rem;
			transition: all 0.2s;

			&:hover {
				background: var(--bg-input);
				color: var(--text-primary);
			}
		}

		.mobile-divider {
			height: 1px;
			background: var(--border-default);
			margin: 0.5rem 0;
		}

		.mobile-btn-primary {
			text-decoration: none;
			background: var(--primary, #ff4655);
			color: white;
			font-weight: 600;
			padding: 0.85rem;
			border-radius: 12px;
			display: flex;
			align-items: center;
			justify-content: center;
			gap: 0.5rem;
			text-align: center;
			transition: all 0.2s;
			box-shadow: 0 4px 15px rgba(255, 70, 85, 0.25);

			&:hover {
				background: #e03e4b;
				transform: translateY(-2px);
			}
		}

		.mobile-shadow-badge {
			background: var(--bg-card);
			border: 1px solid var(--border-default);
			padding: 0.75rem 1rem;
			border-radius: 12px;
			color: var(--text-muted);
			font-size: 0.95rem;
			font-weight: 500;
			display: flex;
			align-items: center;
			justify-content: flex-start;
			gap: 0.75rem;
			cursor: pointer;
			transition: all 0.2s;
			width: 100%;
			text-align: left;
			margin-bottom: 0.5rem;

			&:hover {
				background: var(--bg-card-hover);
				color: var(--text-primary);
				border-color: var(--border-active);
			}
		}
	}

	@media (max-width: 900px) {
		.nav-links.desktop-only {
			display: none;
		}

		.mobile-menu-btn {
			display: flex;
		}

		.landing-nav .left-section .shadow-badge {
			display: none;
		}
	}
</style>
