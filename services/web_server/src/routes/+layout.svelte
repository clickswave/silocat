<script>
	import favicon from '$lib/assets/silo-cat.png';
	import { browser } from '$app/environment';
	import { Toaster } from 'svelte-sonner';
	import { QueryClientProvider, QueryClient } from '@tanstack/svelte-query';
	import '$lib/global.scss';
	import NProgress from 'nprogress';
	import { fade } from 'svelte/transition';
	import 'nprogress/nprogress.css';
	import { navigating } from '$app/stores';

	import { onMount } from 'svelte';
	import { afterNavigate, beforeNavigate } from '$app/navigation';
	import Icon from '@iconify/svelte';
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

	// Track loading state
	let loading = $state(false);
	let nagivateStartTime = $state(Date.now());

	beforeNavigate(async () => {
		loading = true;
	});
	afterNavigate(async () => {
		loading = false;
	});

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

{#if loading}
	<div class="loading-overlay" in:fade={{ duration: 0, delay: 0 }} out:fade={{ duration: 150, delay: 0 }}>
		<Icon icon="svg-spinners:ring-resize" font-size="1.5rem"/>
	</div>
{/if}

<QueryClientProvider client={queryClient}>
	{@render children()}
</QueryClientProvider>

