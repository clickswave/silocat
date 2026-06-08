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
	<div class="nav-inner">
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
				<Icon icon="ri:shield-user-fill" />
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
	</div>
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
						<Icon icon="ri:shield-user-fill" width="20" />
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
	.landing-nav { position: sticky; top: 0; z-index: 50; background: var(--bg-sidebar-glass); backdrop-filter: blur(16px); border-bottom: 1px solid var(--hairline); padding-block: var(--space-3); }
	.nav-inner { display: flex; align-items: center; justify-content: space-between; gap: var(--space-5); width: 100%; max-width: 1180px; margin-inline: auto; padding-inline: var(--gutter); }

	.left-section { display: flex; align-items: center; gap: var(--space-4); }
	.logo { display: flex; align-items: center; gap: var(--space-2); font-weight: var(--fw-black); font-size: 1.2rem; letter-spacing: 0.03em; }
	.logo img { width: 30px; height: 30px; }
	.logo span { color: var(--text-primary); }
	.logo:hover { color: var(--text-primary); }

	.shadow-badge { display: inline-flex; align-items: center; gap: var(--space-2); background: var(--tint-soft); border: 1px solid var(--border-default); padding: 0.4rem 0.8rem; border-radius: var(--radius-pill); color: var(--text-secondary); font-size: var(--fs-sm); font-weight: var(--fw-medium); cursor: pointer; transition: background var(--dur) var(--ease), color var(--dur) var(--ease), border-color var(--dur) var(--ease); }
	.shadow-badge:hover { background: var(--tint-softer); color: var(--text-primary); border-color: var(--border-strong); }
	.shadow-badge :global(svg) { color: var(--primary); }

	.nav-links { display: flex; align-items: center; gap: var(--space-5); }
	.nav-link { position: relative; display: inline-flex; align-items: center; gap: var(--space-2); color: var(--text-secondary); font-size: var(--fs-sm); font-weight: var(--fw-medium); padding: 0.4rem 0; transition: color var(--dur) var(--ease); }
	.nav-link:hover, .nav-link.active { color: var(--text-primary); }
	.nav-link::after { content: ''; position: absolute; left: 0; bottom: -2px; height: 2px; width: 0; background: var(--accent-gradient); border-radius: 2px; transition: width var(--dur) var(--ease); }
	.nav-link:hover::after, .nav-link.active::after { width: 100%; }
	.divider { width: 1px; height: 22px; background: var(--border-default); margin: 0 var(--space-2); }

	.btn-primary { display: inline-flex; align-items: center; gap: var(--space-2); background: var(--accent-gradient); color: #fff; padding: 0.6rem 1.2rem; border-radius: var(--radius-pill); font-weight: var(--fw-semibold); font-size: var(--fs-sm); border: none; box-shadow: 0 6px 18px -6px var(--primary-glow); transition: filter var(--dur) var(--ease), transform var(--dur) var(--ease), box-shadow var(--dur) var(--ease); }
	.btn-primary:hover { filter: brightness(1.06); transform: translateY(-1px); box-shadow: 0 10px 24px -6px var(--primary-glow); color: #fff; }

	.mobile-menu-btn { display: none; background: var(--tint-soft); border: 1px solid var(--border-default); color: var(--text-primary); cursor: pointer; padding: 0.45rem; border-radius: var(--radius-sm); }
	.mobile-menu-btn:hover { background: var(--tint-softer); }

	.modal-backdrop, .mobile-menu-backdrop { position: fixed; inset: 0; background: rgba(0, 0, 0, 0.65); backdrop-filter: blur(8px); z-index: 100; display: flex; align-items: center; justify-content: center; padding: var(--gutter); }
	.modal-content { background: var(--bg-elevated); border: 1px solid var(--border-default); border-radius: var(--radius-lg); padding: var(--space-6); width: 100%; max-width: 480px; display: flex; flex-direction: column; gap: var(--space-5); box-shadow: var(--shadow-lg); }
	.modal-header { display: flex; align-items: center; gap: var(--space-3); }
	.modal-header :global(.modal-icon) { color: var(--primary); }
	.modal-header h2 { font-size: var(--fs-h3); margin: 0; }
	.modal-desc { margin: 0; color: var(--text-secondary); font-size: var(--fs-sm); line-height: var(--lh-normal); }
	.key-display { display: flex; align-items: center; justify-content: space-between; gap: var(--space-3); background: var(--bg-input); border: 1px solid var(--border-default); border-radius: var(--radius-sm); padding: var(--space-4); }
	.key-display code { color: var(--primary); font-family: var(--font-mono); font-size: 0.95rem; overflow: hidden; text-overflow: ellipsis; }
	.key-display .copy-btn { background: transparent; border: none; color: var(--text-secondary); cursor: pointer; padding: var(--space-2); border-radius: var(--radius-sm); display: flex; }
	.key-display .copy-btn:hover { background: var(--tint-softer); color: var(--text-primary); }
	.modal-actions { display: grid; grid-template-columns: 1fr 1fr; gap: var(--space-3); }
	.action-btn { display: flex; align-items: center; justify-content: center; gap: var(--space-2); padding: 0.8rem; border-radius: var(--radius-pill); font-weight: var(--fw-medium); font-size: var(--fs-sm); cursor: pointer; transition: background var(--dur) var(--ease), border-color var(--dur) var(--ease); }
	.action-btn.regenerate { background: var(--tint-soft); border: 1px solid var(--border-default); color: var(--text-primary); }
	.action-btn.regenerate:hover { background: var(--tint-softer); }
	.action-btn.delete { background: rgba(255, 70, 85, 0.1); border: 1px solid rgba(255, 70, 85, 0.25); color: var(--primary); }
	.action-btn.delete:hover { background: rgba(255, 70, 85, 0.18); }
	.close-btn { align-self: flex-end; background: transparent; border: none; color: var(--text-secondary); font-size: var(--fs-sm); cursor: pointer; text-decoration: underline; }
	.close-btn:hover { color: var(--text-primary); }

	.mobile-menu-content { background: var(--bg-elevated); border: 1px solid var(--border-default); border-radius: var(--radius-lg); padding: var(--space-5); width: 100%; max-width: 340px; display: flex; flex-direction: column; gap: var(--space-5); box-shadow: var(--shadow-lg); }
	.mobile-header { display: flex; justify-content: space-between; align-items: center; }
	.menu-title { font-size: var(--fs-h3); font-weight: var(--fw-bold); }
	.close-menu-btn { background: transparent; border: none; color: var(--text-secondary); cursor: pointer; padding: var(--space-1); display: flex; }
	.close-menu-btn:hover { color: var(--text-primary); }
	.mobile-links { display: flex; flex-direction: column; gap: var(--space-2); }
	.mobile-link { display: flex; align-items: center; gap: var(--space-3); color: var(--text-secondary); font-size: var(--fs-body); font-weight: var(--fw-medium); padding: 0.75rem 1rem; border-radius: var(--radius-sm); transition: background var(--dur) var(--ease), color var(--dur) var(--ease); }
	.mobile-link:hover { background: var(--tint-soft); color: var(--text-primary); }
	.mobile-divider { height: 1px; background: var(--border-default); margin: var(--space-2) 0; }
	.mobile-btn-primary { display: flex; align-items: center; justify-content: center; gap: var(--space-2); background: var(--accent-gradient); color: #fff; font-weight: var(--fw-semibold); padding: 0.85rem; border-radius: var(--radius-pill); box-shadow: 0 6px 18px -6px var(--primary-glow); transition: filter var(--dur) var(--ease); }
	.mobile-btn-primary:hover { filter: brightness(1.06); color: #fff; }
	.mobile-shadow-badge { display: flex; align-items: center; gap: var(--space-3); width: 100%; background: var(--tint-soft); border: 1px solid var(--border-default); padding: 0.75rem 1rem; border-radius: var(--radius-sm); color: var(--text-secondary); font-size: var(--fs-body); font-weight: var(--fw-medium); cursor: pointer; transition: background var(--dur) var(--ease), color var(--dur) var(--ease); }
	.mobile-shadow-badge :global(svg) { color: var(--primary); }
	.mobile-shadow-badge:hover { background: var(--tint-softer); color: var(--text-primary); }

	@media (max-width: 900px) {
		.nav-links.desktop-only { display: none; }
		.mobile-menu-btn { display: flex; }
		.shadow-badge { display: none; }
	}
</style>
