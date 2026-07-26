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

	import { onMount } from 'svelte';
	import Version from '$lib/components/Version.svelte';
	import Menu from '$lib/ui/Menu.svelte';

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

		// Optional analytics: only injected when a GA id is configured
		// (PUBLIC_GA_ID) and only on the configured production host, so
		// self-hosters and dev/staging traffic stay untracked by default.
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
			gtag('js', new Date());
			gtag('config', gaId);
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

