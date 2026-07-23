<script>
	import { page } from '$app/stores';
	import Navbar from '$lib/components/Navbar.svelte';
	import Footer from '$lib/components/Footer.svelte';
	import { Button } from '$lib/ui';

	const status = $derived($page.status);
	const message = $derived($page.error?.message || 'Something went wrong');
</script>

<svelte:head>
	<title>{status} - Silocat</title>
	<meta name="robots" content="noindex, nofollow" />
</svelte:head>

<Navbar />

<main class="error-wrap">
	<div class="error-card">
		<p class="code">{status}</p>
		<h1>{status === 404 ? 'Page not found' : 'Something went wrong'}</h1>
		<p class="msg">
			{status === 404
				? 'This page does not exist, or the cat moved it.'
				: message}
		</p>
		<div class="actions">
			<Button href="/">Back to home</Button>
			<Button variant="ghost" href="/pricing">See pricing</Button>
		</div>
	</div>
</main>

<Footer />

<style lang="scss">
	.error-wrap {
		min-height: 60vh;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: var(--space-10) var(--gutter);
	}
	.error-card {
		text-align: center;
		max-width: 440px;
	}
	.code {
		font-family: var(--font-mono);
		font-size: 4rem;
		font-weight: var(--fw-medium);
		line-height: 1;
		margin: 0 0 var(--space-4);
		color: var(--ink-faint);
	}
	h1 {
		font-size: var(--fs-h3);
		margin-bottom: var(--space-2);
	}
	.msg {
		color: var(--ink-mute);
		font-size: var(--fs-sm);
		margin-bottom: var(--space-6);
	}
	.actions {
		display: flex;
		gap: var(--space-3);
		justify-content: center;
		flex-wrap: wrap;
	}
</style>
