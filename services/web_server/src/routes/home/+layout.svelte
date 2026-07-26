<script>
	import Aside from '$lib/components/Aside.svelte';
	import DownloadToasts from '$lib/components/DownloadToasts.svelte';
	import { navigating } from '$app/stores';
	import { afterNavigate } from '$app/navigation';
	import Icon from '$lib/ui/Icon.svelte';

	let { children, data } = $props();

	// Plan chip beside the wordmark. The root layout already resolves the
	// session user, so this needs no fetch of its own. No subscription means
	// Free: that is a real plan here, not an absence, so it is labelled rather
	// than left blank.
	let plan = $derived((data?.user?.subscription?.name || 'free').toLowerCase());

	// Mobile-only off-canvas nav. Close it after navigating to a new page.
	let navOpen = $state(false);
	afterNavigate(() => (navOpen = false));
</script>

<svelte:head>
	<title>Silocat</title>
	<meta name="robots" content="noindex, nofollow" />
</svelte:head>

<!-- The rail is flush to the window edge and owns its own hairline, so the
     shell carries no outer padding or gap: content sits directly beside it. -->
<div class="layout">
	<Aside open={navOpen} onclose={() => (navOpen = false)} />

	{#if navOpen}
		<div class="nav-scrim" role="presentation" onclick={() => (navOpen = false)}></div>
	{/if}

	<DownloadToasts />

	<div class="content-root">
		<header class="topbar">
			<button class="hamburger" onclick={() => (navOpen = true)} aria-label="Open menu">
				<Icon name="menu" size={20} />
			</button>
			<a href="/home" class="topbar-logo">
				<img src="/silocat-logo.png" alt="" width="24" height="24" />
				<span>silocat</span>
			</a>
			<a href="/home/billing" class="plan-chip {plan}" title="Manage your plan">{plan}</a>
		</header>

		<main class="content">
			{@render children()}
		</main>

		{#if $navigating}
			<div class="nav-loading">
				<Icon name="spinner" size={26} />
			</div>
		{/if}
	</div>
</div>

<style lang="scss">
	.layout {
		display: flex;
		gap: 0;
		padding: 0;
		height: 100vh;
		width: 100%;
		overflow: hidden;
		background: var(--bg);
		color: var(--ink);
		font-family: var(--font-sans);
		font-size: var(--fs-body);
		line-height: 1.35;
	}

	.content-root {
		position: relative;
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}

	/* Navigation spinner, scoped to the content pane only (never the rail).
	   Instant show/hide so it never reads as the screen dimming to black. */
	.nav-loading {
		position: absolute;
		inset: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		background: color-mix(in srgb, var(--bg) 55%, transparent);
		color: var(--ink-mute);
		z-index: 20;
	}

	.content {
		flex: 1;
		min-width: 0;
		overflow-y: auto;
		padding: 1rem 1.5rem 1rem 1.25rem;
	}

	/* Mobile top bar + hamburger (hidden on desktop, where the rail is fixed). */
	.topbar {
		display: none;
		align-items: center;
		gap: 0.6rem;
		padding: 0.75rem 1rem 0;
	}

	.hamburger {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 38px;
		height: 38px;
		flex-shrink: 0;
		background: var(--surface);
		border: 1px solid var(--edge);
		border-radius: var(--radius-md);
		color: var(--ink);
		cursor: pointer;
	}

	.topbar-logo {
		display: inline-flex;
		align-items: center;
		gap: var(--space-2);
		text-decoration: none;
		color: var(--ink);

		img {
			width: 24px;
			height: 24px;
			border-radius: var(--radius-sm);
		}
		span {
			font-size: 0.9375rem;
			font-weight: var(--fw-semibold);
			letter-spacing: var(--tracking-tight);
		}
	}

	/* Hairline and flat, per the shell's rules: no pill radius, no fill on the
	   default state. Free reads as quiet; a paid plan takes the accent, which is
	   the one place in the shell it appears. */
	.plan-chip {
		display: inline-flex;
		align-items: center;
		height: 20px;
		padding: 0 var(--space-2);
		margin-left: var(--space-2);
		border: 1px solid var(--edge);
		border-radius: var(--radius-sm);
		font-size: 0.6875rem;
		font-weight: var(--fw-semibold);
		letter-spacing: 0.04em;
		text-transform: uppercase;
		text-decoration: none;
		color: var(--ink-faint);
		transition: color 0.12s ease, border-color 0.12s ease;

		&:hover {
			color: var(--accent);
			border-color: var(--accent);
		}

		&.plus,
		&.pro {
			color: var(--accent);
			border-color: var(--accent);
		}
	}

	.nav-scrim {
		position: fixed;
		inset: 0;
		z-index: 90;
		background: var(--scrim);
	}

	@media (max-width: 768px) {
		.topbar {
			display: flex;
		}
		.content {
			padding: 1rem;
		}
	}
</style>
