<script>
	import Aside from '$lib/components/Aside.svelte';

	let { children } = $props();
</script>

<svelte:head>
	<title>SiloCat: Home</title>
	<meta name="robots" content="noindex, nofollow" />
</svelte:head>

<div class="layout">
	<Aside />

	<div class="content-root">
		<main class="content">
			{@render children()}
		</main>
	</div>
</div>

<style>
	.layout {
		display: flex;
		height: 100vh;
		width: 100%;
		background: var(--bg-app);
	}

	/* Right side wrapper */
	.content-root {
		flex: 1;
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}

	/* Main content area where +page.svelte renders */
	.content {
		flex: 1;
		overflow-y: auto;
		/* Padding is handled by the page content usually, or we can add it here if needed */
		padding: 40px;
		background: var(--bg-app);
	}
</style>
