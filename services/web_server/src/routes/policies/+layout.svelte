<script>
	import Navbar from '$lib/components/Navbar.svelte';
	import Footer from '$lib/components/Footer.svelte';
	import { page } from '$app/stores';

	let { children } = $props();

	const links = [
		{ href: '/policies/terms-of-service', label: 'Terms of Service' },
		{ href: '/policies/acceptable-use', label: 'Acceptable Use Policy' },
		{ href: '/policies/privacy-policy', label: 'Privacy Policy' },
		{ href: '/policies/dmca-policy', label: 'DMCA Policy' },
		{ href: '/policies/refund-policy', label: 'Refund Policy' },
		{ href: '/policies/disclaimer', label: 'Disclaimer' }
	];
</script>

<div class="page">
	<Navbar />

	<main class="main">
		<nav class="rail" aria-label="Legal Center">
			<span class="eyebrow">Legal Center</span>
			<div class="rail-links">
				{#each links as l (l.href)}
					<a href={l.href} class:active={$page.url.pathname === l.href}>{l.label}</a>
				{/each}
			</div>
		</nav>

		<!-- One prose stylesheet, shared by all six documents. The pages carry
		     content only; every heading, list and link style lives here. -->
		<article class="prose">
			{@render children()}
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
		max-width: var(--container);
		margin: 0 auto;
		padding: clamp(2rem, 5vw, 3.5rem) var(--gutter);
		display: flex;
		gap: clamp(2rem, 5vw, 4rem);
		align-items: flex-start;
	}

	.rail {
		position: sticky;
		top: 84px;
		flex: 0 0 200px;
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
	}

	.eyebrow {
		font-family: var(--font-mono);
		font-size: 0.6875rem;
		color: var(--ink-faint);
		text-transform: uppercase;
		letter-spacing: 0.1em;
	}

	.rail-links {
		display: flex;
		flex-direction: column;
		gap: 2px;

		a {
			display: flex;
			align-items: center;
			min-height: 32px;
			padding: 0.375rem 0.625rem;
			border-radius: 8px;
			font-size: var(--fs-sm);
			font-weight: var(--fw-medium);
			color: var(--ink-mute);
			text-decoration: none;
			transition:
				background var(--dur-fast) var(--ease),
				color var(--dur-fast) var(--ease);

			&:hover {
				background: var(--nav-hover);
				color: var(--ink);
			}
			&.active {
				background: var(--accent-soft);
				color: var(--accent);
			}
		}
	}

	.prose {
		flex: 1;
		min-width: 0;
		max-width: 640px;
		display: flex;
		flex-direction: column;
		gap: 1.125rem;
	}

	/* The documents are plain markup, so the stylesheet reaches into them. */
	.prose :global(h1) {
		margin: 0;
		font-size: clamp(1.5rem, 3vw, 2rem);
		font-weight: var(--fw-black);
		letter-spacing: var(--tracking-tight);
		line-height: 1.15;
	}

	.prose :global(h2) {
		margin: var(--space-4) 0 0;
		font-size: 1.25rem;
		font-weight: var(--fw-semibold);
		letter-spacing: var(--tracking-tight);
	}

	.prose :global(h3) {
		margin: var(--space-2) 0 0;
		font-size: 1rem;
		font-weight: var(--fw-semibold);
		letter-spacing: var(--tracking-tight);
	}

	.prose :global(p) {
		margin: 0;
		color: var(--ink-mute);
		text-wrap: pretty;
	}

	.prose :global(ul),
	.prose :global(ol) {
		margin: 0;
		padding-left: 1.25rem;
		color: var(--ink-mute);
		display: flex;
		flex-direction: column;
		gap: 0.375rem;
	}

	.prose :global(ul ul),
	.prose :global(ol ol),
	.prose :global(ul ol),
	.prose :global(ol ul) {
		margin: 0.375rem 0 0;
		gap: var(--space-1);
	}

	.prose :global(a) {
		color: var(--accent);
		text-decoration: none;

		&:hover {
			color: var(--accent-hover);
			text-decoration: underline;
		}
	}

	.prose :global(strong) {
		color: var(--ink);
		font-weight: var(--fw-semibold);
	}

	.prose :global(code) {
		font-family: var(--font-mono);
		font-size: 0.875em;
		color: var(--ink);
	}

	/* "Last updated" line, and the contact card the documents close with. */
	.prose :global(.updated) {
		font-family: var(--font-mono);
		font-size: var(--fs-xs);
		color: var(--ink-faint);
	}

	.prose :global(.contact) {
		margin-top: var(--space-4);
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
		padding: 1rem;
		border: 1px solid var(--edge);
		border-radius: var(--radius-md);
		background: var(--surface);
		font-size: var(--fs-sm);
		color: var(--ink-mute);
	}

	.prose :global(.contact strong) {
		font-size: var(--fs-sm);
	}

	@media (max-width: 860px) {
		.main {
			flex-direction: column;
			gap: var(--space-6);
		}
		.rail {
			position: static;
			flex: 1 1 auto;
			width: 100%;
		}
		.rail-links {
			flex-direction: row;
			flex-wrap: wrap;
			gap: var(--space-1);
		}
	}
</style>
