<script>
	import { browser } from '$app/environment';
	import { SITE, organizationSchema, websiteSchema } from '$lib/seo.js';
	import { Toaster } from 'svelte-sonner';
	import { QueryClientProvider, QueryClient } from '@tanstack/svelte-query';
	import '$lib/global.scss';
	import NProgress from 'nprogress';
	import 'nprogress/nprogress.css';
	import { navigating } from '$app/stores';

	import { onMount } from 'svelte';
	import Version from '$lib/components/Version.svelte';

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
	{@html `<script type="application/ld+json">${JSON.stringify(organizationSchema())}<\/script>`}
	{@html `<script type="application/ld+json">${JSON.stringify(websiteSchema())}<\/script>`}
</svelte:head>

<Toaster position="top-center" richColors theme="dark" />

<QueryClientProvider client={queryClient}>
	{@render children()}
</QueryClientProvider>

