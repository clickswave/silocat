<script>
	import Navbar from '$lib/components/Navbar.svelte';
	import Footer from '$lib/components/Footer.svelte';
	import { page } from '$app/stores';

	const links = [
		{ href: '/policies/terms-of-service', label: 'Terms of Service' },
		{ href: '/policies/acceptable-use', label: 'Acceptable Use Policy' },
		{ href: '/policies/privacy-policy', label: 'Privacy Policy' },
		{ href: '/policies/dmca-policy', label: 'DMCA Policy' },
		{ href: '/policies/refund-policy', label: 'Refund Policy' },
		{ href: '/policies/disclaimer', label: 'Disclaimer' }
	];
</script>

<div class="policy-layout">
	<Navbar />

	<div class="content-wrapper">
		<aside class="sidebar">
			<h3 class="eyebrow">Legal Center</h3>
			<div class="links">
				{#each links as link}
					<a href={link.href} class:active={$page.url.pathname === link.href}>
						{link.label}
					</a>
				{/each}
			</div>
		</aside>

		<main class="main-content">
			<div class="container narrow policy-prose">
				<slot />
			</div>
		</main>
	</div>

	<Footer />
</div>

<div class="bg-effects">
	<div class="glow-spot top"></div>
</div>

<style lang="scss">
	.policy-layout {
		position: relative;
		z-index: 1;
		min-height: 100vh;
		display: flex;
		flex-direction: column;
	}

	.content-wrapper {
		display: flex;
		flex: 1;
		width: 100%;
		max-width: var(--container-wide);
		margin-inline: auto;
		position: relative;
		z-index: 10;
		gap: var(--space-6);
		padding-inline: var(--gutter);

		@media (max-width: 860px) {
			flex-direction: column;
			gap: 0;
			padding-inline: 0;
		}
	}

	.sidebar {
		flex: none;
		width: 280px;
		padding-block: var(--space-10) var(--space-6);

		@media (max-width: 860px) {
			width: 100%;
			border-bottom: 1px solid var(--hairline);
			padding: var(--space-5) var(--gutter);
		}

		h3 {
			margin-bottom: var(--space-5);
		}

		.links {
			display: flex;
			flex-direction: column;
			gap: var(--space-1);

			@media (max-width: 860px) {
				flex-direction: row;
				flex-wrap: wrap;
				gap: var(--space-2);
			}

			a {
				color: var(--text-secondary);
				padding: var(--space-3) var(--space-4);
				border: 1px solid transparent;
				border-radius: var(--radius-sm);
				transition:
					color var(--dur) var(--ease),
					background var(--dur) var(--ease),
					border-color var(--dur) var(--ease);
				font-size: var(--fs-sm);
				font-weight: var(--fw-medium);

				&:hover {
					color: var(--text-primary);
					background: var(--tint-soft);
				}

				&.active {
					background: var(--tint-soft);
					border-color: var(--border-default);
					color: var(--primary);
				}
			}
		}
	}

	.main-content {
		flex: 1;
		min-width: 0;
		padding-block: var(--space-10) var(--space-12);

		@media (max-width: 860px) {
			padding-block: var(--space-6) var(--space-10);
		}
	}

	.policy-prose {
		margin-inline: 0;
		padding-inline: 0;

		@media (max-width: 860px) {
			padding-inline: var(--gutter);
		}
	}

	/* ---- shared prose styles applied to every policy page ---- */
	.policy-prose :global(.policy-document) {
		display: flex;
		flex-direction: column;
	}

	.policy-prose :global(h1) {
		font-size: var(--fs-h1);
		font-weight: var(--fw-black);
		margin-bottom: var(--space-2);
	}

	.policy-prose :global(.last-updated) {
		color: var(--text-muted);
		font-size: var(--fs-sm);
		font-family: var(--font-mono);
		margin: 0 0 var(--space-8);
	}

	.policy-prose :global(section) {
		margin-bottom: var(--space-8);

		&:last-child {
			margin-bottom: 0;
		}
	}

	.policy-prose :global(section h2) {
		font-size: var(--fs-h3);
		font-weight: var(--fw-semibold);
		margin-bottom: var(--space-4);
	}

	.policy-prose :global(section h2)::before {
		content: '';
		display: inline-block;
		width: 3px;
		height: 0.9em;
		margin-right: var(--space-3);
		vertical-align: -0.05em;
		border-radius: var(--radius-sm);
		background: var(--accent-gradient);
	}

	.policy-prose :global(p) {
		color: var(--text-secondary);
		line-height: var(--lh-normal);
		margin-bottom: var(--space-4);

		&:last-child {
			margin-bottom: 0;
		}
	}

	.policy-prose :global(strong) {
		color: var(--text-primary);
		font-weight: var(--fw-semibold);
	}

	.policy-prose :global(ul) {
		color: var(--text-secondary);
		line-height: var(--lh-normal);
		margin: 0 0 var(--space-4);
		padding-left: var(--space-5);
	}

	.policy-prose :global(li) {
		margin-bottom: var(--space-2);
	}

	.policy-prose :global(li)::marker {
		color: var(--primary);
	}

	.policy-prose :global(code) {
		font-family: var(--font-mono);
		font-size: 0.9em;
		background: var(--tint-soft);
		border: 1px solid var(--hairline);
		border-radius: var(--radius-sm);
		padding: 0.1em 0.4em;
		color: var(--text-primary);
	}

	.policy-prose :global(a) {
		color: var(--primary);
		text-decoration: none;
		transition: color var(--dur) var(--ease);
	}

	.policy-prose :global(a:hover) {
		color: var(--primary-hover);
		text-decoration: underline;
		text-underline-offset: 3px;
	}

	/* ---- background fx ---- */
	.bg-effects {
		position: fixed;
		inset: 0;
		z-index: 0;
		pointer-events: none;
		overflow: hidden;
	}
	.glow-spot {
		position: absolute;
		width: 600px;
		height: 600px;
		filter: blur(110px);
		border-radius: 50%;
	}
	.glow-spot.top {
		top: -22%;
		left: 12%;
		background: radial-gradient(circle, rgba(255, 70, 85, 0.12) 0%, transparent 70%);
	}
</style>
