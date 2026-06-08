<script>
	import Aside from '$lib/components/Aside.svelte';
	import DownloadToasts from '$lib/components/DownloadToasts.svelte';
	import SiloCatLogo from '$lib/assets/silo-cat.png';
	import { navigating } from '$app/stores';
	import { afterNavigate } from '$app/navigation';
	import Icon from '@iconify/svelte';

	let { children } = $props();

	// Mobile-only off-canvas nav. Close it after navigating to a new page.
	let navOpen = $state(false);
	afterNavigate(() => (navOpen = false));
</script>

<svelte:head>
	<title>SiloCat: Home</title>
	<meta name="robots" content="noindex, nofollow" />
</svelte:head>

<div class="layout">
	<Aside open={navOpen} onclose={() => (navOpen = false)} />

	{#if navOpen}
		<div class="nav-scrim" role="presentation" onclick={() => (navOpen = false)}></div>
	{/if}

	<DownloadToasts />

	<div class="content-root">
		<header class="topbar">
			<button class="hamburger" onclick={() => (navOpen = true)} aria-label="Open menu">
				<Icon icon="ri:menu-line" width="22" />
			</button>
			<a href="/home" class="topbar-logo">
				<img src={SiloCatLogo} alt="SiloCat" />
				<span>SILO.CAT</span>
			</a>
		</header>

		<main class="content">
			<div class="content-inner">
				{@render children()}
			</div>
		</main>

		{#if $navigating}
			<div class="nav-loading">
				<Icon icon="svg-spinners:ring-resize" font-size="2rem" />
			</div>
		{/if}
	</div>
</div>

<style>
	.layout {
		display: flex;
		gap: 1rem;
		height: 100vh;
		width: 100%;
		padding: 1rem;
		box-sizing: border-box;
		background: var(--bg-app);
		color: var(--text-primary);
		font-family: var(--font-sans);
	}

	/* Right side wrapper */
	.content-root {
		position: relative;
		flex: 1;
		display: flex;
		flex-direction: column;
		overflow: hidden;
		min-width: 0;
	}

	/* Navigation spinner, scoped to the content pane only (never the sidebar).
	   Instant show/hide (no fade) so it never reads as a screen dimming to black. */
	.nav-loading {
		position: absolute;
		inset: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		background: color-mix(in srgb, var(--bg-app) 55%, transparent);
		color: var(--text-primary);
		z-index: 20;
	}

	/* Main content area where +page.svelte renders. Padding lives on the inner
	   wrapper, not this scroll container: scroll containers clip their bottom
	   padding (notably in Firefox), which made the top gap look larger than the
	   bottom. The inner wrapper renders symmetric padding reliably. */
	.content {
		flex: 1;
		overflow-y: auto;
		background: var(--bg-app);
	}

	/* Content fills the pane. The app's 1rem shell padding + the flex gap are the
	   only outer spacing, so every page lines up identically and stays wide. */
	.content-inner {
		width: 100%;
	}

	/* Mobile top bar + hamburger (hidden on desktop, where the sidebar is fixed). */
	.topbar {
		display: none;
		align-items: center;
		gap: 0.6rem;
		margin-bottom: 0.85rem;
	}
	.hamburger {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 40px;
		height: 40px;
		flex-shrink: 0;
		background: var(--bg-card);
		border: 1px solid var(--border-sidebar);
		border-radius: var(--radius-md);
		color: var(--text-primary);
		cursor: pointer;
	}
	.topbar-logo {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		text-decoration: none;
		color: var(--text-primary);
	}
	.topbar-logo img {
		width: 26px;
		height: 26px;
	}
	.topbar-logo span {
		font-weight: var(--fw-black);
		letter-spacing: 0.03em;
	}

	.nav-scrim {
		position: fixed;
		inset: 0;
		z-index: 90;
		background: rgba(0, 0, 0, 0.5);
	}

	@media (max-width: 768px) {
		.layout {
			padding: 0.75rem;
			gap: 0;
		}
		.topbar {
			display: flex;
		}
	}
</style>
