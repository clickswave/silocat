<script>
	import Icon from '@iconify/svelte';
	import Version from '$lib/components/Version.svelte';
</script>

<footer class="footer">
	<div class="footer-inner">
		<p class="copy">© 2026 SiloCat · by Clickswave</p>

		<nav class="links" aria-label="Legal">
			<a href="/policies/terms-of-service">Terms</a>
			<a href="/policies/privacy-policy">Privacy</a>
			<a href="/policies/acceptable-use">Usage</a>
			<a href="/policies/dmca-policy">DMCA</a>
			<a href="/policies/refund-policy">Refunds</a>
			<a href="/policies/disclaimer">Disclaimer</a>
		</nav>

		<div class="social">
			<a href="https://twitter.com/clickswave" target="_blank" rel="noreferrer" aria-label="X"><Icon icon="ri:twitter-x-line" width="18" /></a>
			<a href="https://github.com/clickswave" target="_blank" rel="noreferrer" aria-label="GitHub"><Icon icon="ri:github-fill" width="18" /></a>
			<a href="https://discord.gg/clickswave" target="_blank" rel="noreferrer" aria-label="Discord"><Icon icon="ri:discord-fill" width="18" /></a>
		</div>
	</div>
	<Version version="0.0.1" />
</footer>

<style lang="scss">
	.footer {
		border-top: 1px solid var(--hairline);
		padding: var(--space-5) var(--gutter);
		margin-top: auto;
	}
	.footer-inner {
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
		color: var(--text-muted);
		font-size: var(--fs-sm);
	}
	.links {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-4);
	}
	.links a {
		color: var(--text-secondary);
		font-size: var(--fs-sm);
	}
	.links a:hover {
		color: var(--text-primary);
	}
	.social {
		display: flex;
		gap: var(--space-3);
	}
	.social a {
		display: flex;
		color: var(--text-muted);
	}
	.social a:hover {
		color: var(--primary);
	}
	@media (max-width: 640px) {
		.footer-inner {
			flex-direction: column;
			align-items: flex-start;
			gap: var(--space-3);
		}
	}
</style>
