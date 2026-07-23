<script>
	import Icon from '@iconify/svelte';

	const year = new Date().getFullYear();
</script>

<footer class="footer">
	<div class="inner">
		<p class="copy">
			© {year} Silocat · <a href="https://clickswave.org" target="_blank" rel="noreferrer">Clickswave</a>
			<span class="sep">·</span>
			<a href="https://github.com/clickswave/silocat" target="_blank" rel="noreferrer">Open source, AGPL-3.0</a>
		</p>

		<nav class="links" aria-label="Legal">
			<a href="/policies/terms-of-service">Terms</a>
			<a href="/policies/privacy-policy">Privacy</a>
			<a href="/policies/acceptable-use">Usage</a>
			<a href="/policies/dmca-policy">DMCA</a>
			<a href="/policies/refund-policy">Refunds</a>
			<a href="/policies/disclaimer">Disclaimer</a>
		</nav>

		<div class="social">
			<a href="https://twitter.com/clickswave" target="_blank" rel="noreferrer" aria-label="X">
				<Icon icon="ri:twitter-x-line" width="16" />
			</a>
			<a href="https://github.com/clickswave/silocat" target="_blank" rel="noreferrer" aria-label="GitHub">
				<Icon icon="ri:github-fill" width="16" />
			</a>
		</div>
	</div>
</footer>

<style lang="scss">
	.footer {
		border-top: 1px solid var(--edge);
		padding: var(--space-5) var(--gutter);
		margin-top: auto;
	}
	.inner {
		max-width: var(--container);
		margin-inline: auto;
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-5);
		flex-wrap: wrap;
	}
	.copy {
		margin: 0;
		color: var(--ink-faint);
		font-size: var(--fs-sm);

		a {
			color: var(--ink-mute);
			&:hover {
				color: var(--ink);
			}
		}
		.sep {
			margin-inline: var(--space-1);
		}
	}
	.links {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-4);

		a {
			color: var(--ink-faint);
			font-size: var(--fs-sm);
			&:hover {
				color: var(--ink);
			}
		}
	}
	.social {
		display: flex;
		gap: var(--space-2);

		a {
			display: flex;
			padding: var(--space-1);
			color: var(--ink-faint);
			&:hover {
				color: var(--ink);
			}
		}
	}
	@media (max-width: 640px) {
		.inner {
			flex-direction: column;
			align-items: flex-start;
			gap: var(--space-3);
		}
	}
</style>
