<script>
	import favicon from '$lib/assets/silo-cat.png';
	import { browser } from '$app/environment';
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
	<link rel="icon" href={favicon} />
	<title>SiloCat</title>
</svelte:head>

<Toaster position="top-center" richColors theme="dark" />

<QueryClientProvider client={queryClient}>
	{@render children()}
</QueryClientProvider>

