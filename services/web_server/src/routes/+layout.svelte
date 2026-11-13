<script>
	import { browser } from '$app/environment';
	import { env } from '$env/dynamic/public';
	import { SITE, organizationSchema, websiteSchema } from '$lib/seo.js';
	import { theme } from '$lib/theme.js';
	import { Toaster } from 'svelte-sonner';
	import { QueryClientProvider, QueryClient } from '@tanstack/svelte-query';
	import '$lib/global.scss';
	import NProgress from 'nprogress';
	import 'nprogress/nprogress.css';
	import { navigating } from '$app/stores';
	import { beforeNavigate } from '$app/navigation';

	import { onMount } from 'svelte';
	import Version from '$lib/components/Version.svelte';
	import Menu from '$lib/ui/Menu.svelte';
	import { captureSource } from '$lib/source.js';
	import { forcesFullLoad, isLinkPage, separateLinkPages } from '$lib/analytics.js';
	import { confirmFullLoad } from '$lib/transferGuard.js';

	NProgress.configure({
		showSpinner: false,
		speed: 300
	});

	let { children } = $props();
	const queryClient = new QueryClient();

	// Show progress bar on initial load (hydration)
	if (browser) {
		NProgress.start();
	}

	onMount(() => {
		NProgress.done();
		captureSource();

		// Optional analytics: only injected when a GA id is configured
		// (PUBLIC_GA_ID) and only on the configured production host, so
		// self-hosters and dev/staging traffic stay untracked by default.
		// Build-time gate: false under `vite dev`, so a dev server cannot report
		// even if it is reached on a matching hostname.
		if (!import.meta.env.PROD) return;
		const gaId = env.PUBLIC_GA_ID;
		if (gaId && location.hostname === (env.PUBLIC_GA_HOSTNAME || 'silo.cat')) {
			// GA runs on this site, but never on a drop, share or request page,
			// whose URL is the link itself, and moving onto or off one of those
			// is a full page load. See $lib/analytics.js.
			separateLinkPages();
			if (isLinkPage(location.pathname)) return;
			const gaScript = document.createElement('script');
			gaScript.async = true;
			gaScript.src = `https://www.googletagmanager.com/gtag/js?id=${gaId}`;
			document.head.appendChild(gaScript);
			window.dataLayer = window.dataLayer || [];
			function gtag() {
				window.dataLayer.push(arguments);
			}
			// Publish the shim so `$lib/track.js` can send events. gtag.js does not
			// define this global itself; the standard snippet does, and this is our
			// copy of it. Without the assignment every event call is a silent no-op.
			/** @type {any} */ (window).gtag = gtag;
			gtag('js', new Date());
			gtag('config', gaId);
		}
	});

	// While GA runs, moving onto or off a link page loads the next page in full.
	// gtag.js reports every History API change by itself, with the real URL, so
	// an in-place move onto a link page would hand GA the link, and after an
	// in-place move off one GA would not be running. Back and Forward never cross
	// that line in place, because no in-place move across it is ever made. A
	// running transfer ends with the page, so confirmFullLoad asks first.
	beforeNavigate((nav) => {
		if (nav.willUnload || !nav.to || !forcesFullLoad(nav.from?.url, nav.to.url)) return;
		nav.cancel();
		if (confirmFullLoad()) location.href = nav.to.url.href;
	});

	$effect(() => {
		if ($navigating) {
			NProgress.start();
		} else {
			NProgress.done();
		}
	});
</script>

<svelte:head>
	<title>{SITE.defaultTitle}</title>
	{@html `<script type="application/ld+json">${JSON.stringify(organizationSchema()).replace(/</g, '\\u003c')}<\/script>`}
	{@html `<script type="application/ld+json">${JSON.stringify(websiteSchema()).replace(/</g, '\\u003c')}<\/script>`}
</svelte:head>

<!-- Bottom-right, token-skinned (see global.scss). `richColors` is off on
     purpose: it paints its own backgrounds and fights the one-accent rule. -->
<Toaster position="bottom-right" theme={$theme} closeButton />
<Menu />

<QueryClientProvider client={queryClient}>
	{@render children()}
</QueryClientProvider>

