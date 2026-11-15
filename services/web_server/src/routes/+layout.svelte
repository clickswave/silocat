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
	import { leaveLinkPagesInFull, startAnalytics } from '$lib/analytics.js';
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

		// GA runs only in a production build with PUBLIC_GA_ID set, on the
		// configured host, and never on a drop, share or request page, whose URL
		// is the link itself. Moving onto or off one of those is a full page load,
		// and confirmFullLoad asks first while a transfer runs, because the
		// transfer ends with the page. See $lib/analytics.js.
		startAnalytics({
			prod: import.meta.env.PROD,
			gaId: env.PUBLIC_GA_ID,
			gaHostname: env.PUBLIC_GA_HOSTNAME,
			confirm: confirmFullLoad
		});
	});

	beforeNavigate(leaveLinkPagesInFull);

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

