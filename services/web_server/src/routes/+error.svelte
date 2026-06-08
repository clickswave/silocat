<script>
	import { page } from '$app/stores';
	import Navbar from '$lib/components/Navbar.svelte';
	import Footer from '$lib/components/Footer.svelte';

	const status = $derived($page.status);
	const message = $derived($page.error?.message || 'Something went wrong');
</script>

<svelte:head>
	<title>{status} - SiloCat</title>
	<meta name="robots" content="noindex, nofollow" />
</svelte:head>

<Navbar />

<main class="error-wrap">
	<div class="error-card">
		<p class="code">{status}</p>
		<h1>{status === 404 ? 'Page not found' : 'Something went wrong'}</h1>
		<p class="msg">
			{status === 404
				? 'The page you are looking for does not exist or may have been moved.'
				: message}
		</p>
		<div class="actions">
			<a class="btn primary" href="/">Back to home</a>
			<a class="btn ghost" href="/pricing">View pricing</a>
		</div>
	</div>
</main>

<Footer />

<style>
	.error-wrap {
		min-height: 60vh;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 4rem 1rem;
		color: var(--text-primary, #e9e9ee);
	}
	.error-card {
		text-align: center;
		max-width: 440px;
	}
	.code {
		font-size: 5rem;
		font-weight: 800;
		line-height: 1;
		margin: 0;
		background: linear-gradient(90deg, #ff4655, #ff8a93);
		-webkit-background-clip: text;
		background-clip: text;
		color: transparent;
	}
	h1 {
		font-size: 1.6rem;
		font-weight: 700;
		margin: 0.75rem 0 0.5rem;
	}
	.msg {
		color: var(--text-muted, #9a9aa3);
		margin: 0 0 1.75rem;
	}
	.actions {
		display: flex;
		gap: 0.75rem;
		justify-content: center;
		flex-wrap: wrap;
	}
	.btn {
		padding: 0.65rem 1.25rem;
		border-radius: 8px;
		font-weight: 600;
		text-decoration: none;
		font-size: 0.9rem;
	}
	.btn.primary {
		background: linear-gradient(90deg, #ff4655, #ff8a93);
		color: #fff;
	}
	.btn.ghost {
		border: 1px solid var(--border-default, #2e2e35);
		color: var(--text-primary, #e9e9ee);
	}
</style>
