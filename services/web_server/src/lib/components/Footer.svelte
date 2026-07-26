<script>
	import Icon from '$lib/ui/Icon.svelte';

	const year = new Date().getFullYear();

	const legal = [
		{ href: '/policies/terms-of-service', label: 'Terms' },
		{ href: '/policies/privacy-policy', label: 'Privacy' },
		{ href: '/policies/acceptable-use', label: 'Usage' },
		{ href: '/policies/dmca-policy', label: 'DMCA' },
		{ href: '/policies/refund-policy', label: 'Refunds' },
		{ href: '/policies/disclaimer', label: 'Disclaimer' }
	];
</script>

<footer class="footer">
	<div class="inner">
		<!-- The Clickswave and AGPL links are styled to read as plain text at rest,
		     matching the design, but stay real links: showing the licence and
		     pointing at the source is a promise the product makes on every page. -->
		<span class="copy">
			© {year} Silocat ·
			<a href="https://clickswave.org" target="_blank" rel="noreferrer">Clickswave</a> ·
			<a href="https://github.com/clickswave/silocat" target="_blank" rel="noreferrer">
				Open source, AGPL-3.0
			</a>
		</span>

		<nav class="links" aria-label="Legal">
			{#each legal as l (l.href)}
				<a href={l.href}>{l.label}</a>
			{/each}
		</nav>

		<div class="social">
			<a href="https://twitter.com/clickswave" target="_blank" rel="noreferrer" aria-label="X">
				<Icon name="x" size={14} />
			</a>
			<a
				href="https://github.com/clickswave/silocat"
				target="_blank"
				rel="noreferrer"
				aria-label="GitHub"
			>
				<Icon name="github" size={15} />
			</a>
		</div>
	</div>
</footer>

<style lang="scss">
	.footer {
		border-top: 1px solid var(--edge);
		font-family: var(--font-sans);
	}

	.inner {
		max-width: var(--container);
		margin: 0 auto;
		padding: 1.25rem var(--gutter);
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-4);
	}

	.copy {
		font-size: var(--fs-sm);
		color: var(--ink-faint);

		a {
			color: inherit;
			text-decoration: none;
			transition: color var(--dur-fast) var(--ease);

			&:hover {
				color: var(--ink);
			}
		}
	}

	.links {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-4);

		a {
			font-size: var(--fs-sm);
			color: var(--ink-faint);
			text-decoration: none;
			transition: color var(--dur-fast) var(--ease);

			&:hover {
				color: var(--ink);
			}
		}
	}

	.social {
		display: flex;
		gap: var(--space-1);

		a {
			width: 28px;
			height: 28px;
			border-radius: var(--radius-sm);
			display: grid;
			place-items: center;
			color: var(--ink-faint);
			transition:
				background var(--dur-fast) var(--ease),
				color var(--dur-fast) var(--ease);

			&:hover {
				background: var(--nav-hover);
				color: var(--ink);
			}
		}
	}
</style>
