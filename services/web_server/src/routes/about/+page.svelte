<script>
	import Navbar from '$lib/components/Navbar.svelte';
	import Footer from '$lib/components/Footer.svelte';
	import Seo from '$lib/components/Seo.svelte';
	import { breadcrumbSchema } from '$lib/seo.js';
</script>

<Seo
	title="About Silocat: Privacy-first encrypted storage"
	description="Silocat's mission is private, zero-knowledge file sharing and cloud storage. Learn how end-to-end encryption keeps your files yours, and yours alone."
	schema={breadcrumbSchema([
		{ name: 'Home', path: '/' },
		{ name: 'About', path: '/about' }
	])}
/>

<div class="page-container">
	<Navbar />

	<main class="content">
		<section class="section">
			<div class="container narrow">
				<h1>About</h1>

				<div class="prose">
					<p>
						Silocat is a zero-knowledge file service. Files are encrypted in your browser before
						they leave your device; what reaches our servers is ciphertext we cannot read. Sharing
						works without an account, and downloads pull chunks in parallel so big files arrive
						fast.
					</p>
					<p>
						Storage is replicated across independent cloud providers, watched over by an internal
						sentinel that keeps uploads healthy and reclaims anything orphaned. You see a link;
						the plumbing stays out of the way.
					</p>
					<p>
						Silocat is open source under AGPL-3.0. You can
						<a href="https://github.com/clickswave/silocat" target="_blank" rel="noreferrer">read every line</a>
						of the code that handles your files, or run the whole thing on your own hardware.
					</p>
					<p>
						Privacy is not lawlessness. We comply with DMCA and applicable law, and abuse gets
						accounts closed. We protect your files from prying eyes, not from consequences.
					</p>
					<p class="byline">
						Built by <a href="https://clickswave.org" target="_blank" rel="noreferrer">Clickswave Labs</a>.
					</p>
				</div>
			</div>
		</section>
	</main>

	<Footer />
</div>

<style lang="scss">
	.page-container {
		min-height: 100vh;
		display: flex;
		flex-direction: column;
	}

	.content {
		flex: 1;
	}

	h1 {
		font-size: var(--fs-h1);
		margin-bottom: var(--space-6);
	}

	.prose {
		display: flex;
		flex-direction: column;
		gap: var(--space-4);

		p {
			color: var(--ink-mute);
			line-height: var(--lh-normal);
			font-size: var(--fs-body);
		}

		a {
			color: var(--ink);
			text-decoration: underline;
			text-underline-offset: 3px;

			&:hover {
				color: var(--accent);
			}
		}

		.byline {
			margin-top: var(--space-4);
			font-size: var(--fs-sm);
			color: var(--ink-faint);
		}
	}
</style>
