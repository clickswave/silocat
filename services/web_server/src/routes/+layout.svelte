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
	import { analyticsConfig, needsFullLoad } from '$lib/analytics.js';

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

	// Set once gtag.js has been loaded into this page.
	let gaLoaded = false;

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
			// Not the bare config: on a drop, share or request page that sends GA
			// the link itself and, on a drop, the file's name. See $lib/analytics.js.
			gtag('config', gaId, analyticsConfig({ href: location.href, referrer: document.referrer }));
			gaLoaded = true;
		}
	});

	// gtag.js reports every History API change by itself, with the real new URL
	// and the real one before it, so moving onto or off a drop, share or request
	// page in place would hand GA the link after all. Those moves load the next
	// page in full instead, and the fresh gtag.js there only hears the config
	// above. Back and Forward never cross that line in place, because no in-place
	// move across it is ever made.
	beforeNavigate((nav) => {
		if (!gaLoaded || nav.willUnload || !nav.to) return;
		if (needsFullLoad(nav.from?.url, nav.to.url)) {
			nav.cancel();
			location.href = nav.to.url.href;
		}
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

