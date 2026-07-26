<script>
	import Seo from '$lib/components/Seo.svelte';
	import { breadcrumbSchema } from '$lib/seo.js';
	import Navbar from '$lib/components/Navbar.svelte';
	import Footer from '$lib/components/Footer.svelte';
</script>

<Seo
	title="About Silocat: Privacy-first encrypted storage"
	description="Silocat's mission is private, zero-knowledge file sharing and cloud storage. Learn how end-to-end encryption keeps your files yours, and yours alone."
	schema={breadcrumbSchema([
		{ name: 'Home', path: '/' },
		{ name: 'About', path: '/about' }
	])}
/>

<div class="page">
	<Navbar />

	<main class="main">
		<article class="prose">
			<h1>About</h1>

			<p>
				Silocat exists because sending a file should not require a subscription, an account, or a
				privacy policy you have to trust. You drop a file, it gets encrypted in your browser, and
				you get a link. That's the whole product.
			</p>
			<p>
				The name is honest about what it is: a silo, guarded by a cat. Cats do not care about your
				files. They will not read them, sell them, or train anything on them. Neither will we, and
				unlike a promise, that's enforced by mathematics. The server stores ciphertext it cannot
				open.
			</p>
			<p>
				Every feature is available on every tier. Free accounts get the same encryption, the same
				share controls, the same speed. Paying gets you more space, nothing else. We think
				paywalling security features is a moral failure, so we don't.
			</p>
			<p>
				The entire codebase is open source under AGPL-3.0. You can read the encryption, audit the
				server, or run the whole thing on your own hardware for free, forever. If we ever became
				untrustworthy, you could simply leave and take the software with you. That's the point.
			</p>
			<p>
				Silocat is built and run by a small team that would rather keep it simple than make it big.
				No growth hacks, no dark patterns, no emails you didn't ask for.
			</p>

			<p class="byline">
				A <a href="https://clickswave.org" target="_blank" rel="noreferrer">Clickswave</a> project ·
				<a href="https://github.com/clickswave/silocat" target="_blank" rel="noreferrer">
					source on GitHub
				</a>
			</p>
		</article>
	</main>

	<Footer />
</div>

<style lang="scss">
	.page {
		min-height: 100vh;
		display: flex;
		flex-direction: column;
		background: var(--bg);
		color: var(--ink);
		font-family: var(--font-sans);
		font-size: var(--fs-body);
		line-height: var(--lh-normal);
	}

	.main {
		flex: 1;
		width: 100%;
		max-width: var(--container-narrow);
		margin: 0 auto;
		padding-inline: var(--gutter);
	}

	.prose {
		padding: clamp(2.5rem, 7vw, 4.5rem) 0 clamp(3rem, 8vw, 5rem);
		display: flex;
		flex-direction: column;
		gap: 1.25rem;

		h1 {
			margin: 0 0 var(--space-2);
			font-size: var(--fs-h1);
			font-weight: var(--fw-black);
			letter-spacing: var(--tracking-tight);
			line-height: var(--lh-tight);
		}

		p {
			margin: 0;
			/* This page is pure typography, so the body sits a notch above the
			   app's 15px default for a more comfortable reading measure. */
			font-size: 1.0625rem;
			line-height: 1.65;
			color: var(--ink-mute);
			text-wrap: pretty;
		}
	}

	.byline {
		margin-top: var(--space-4) !important;
		font-size: 0.9375rem !important;
		color: var(--ink-faint) !important;

		/* Accent at rest, not only on hover: these two are the page's only
		   outbound links and should read as links without being hunted for. */
		a {
			color: var(--accent);
			font-weight: var(--fw-medium);
			text-decoration: none;

			&:hover {
				color: var(--accent-hover);
				text-decoration: underline;
				text-underline-offset: 3px;
			}
		}
	}
</style>
