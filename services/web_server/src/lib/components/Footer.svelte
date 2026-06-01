<script>
	import Icon from '@iconify/svelte';
	import ClickswaveLogo from '$lib/assets/clickswave_transparent.webp';
	import Version from '$lib/components/Version.svelte';
</script>

<footer class="landing-footer">
	<div class="footer-content">
		<div class="brand-section">
			<div class="administered-by">
				<span>Administered by</span>
				<div class="brand-name">
					<img src={ClickswaveLogo} alt="Clickswave" class="cw-logo" />
					<strong>CLICKSWAVE</strong>
				</div>
			</div>
			<p class="copyright">
				© 2026 SiloCat. Cat powered file sharing and storage platform.
			</p>
		</div>

		<div class="footer-right">
			<nav class="legal-nav" aria-label="Legal">
				<a href="/policies/terms-of-service">
					<Icon icon="ri:file-text-line" width="16" />
					<span>Terms</span>
				</a>
				<a href="/policies/privacy-policy">
					<Icon icon="ri:shield-check-line" width="16" />
					<span>Privacy</span>
				</a>
				<a href="/policies/acceptable-use">
					<Icon icon="ri:checkbox-circle-line" width="16" />
					<span>Usage</span>
				</a>
				<a href="/policies/dmca-policy">
					<Icon icon="ri:copyright-line" width="16" />
					<span>DMCA</span>
				</a>
				<a href="/policies/refund-policy">
					<Icon icon="ri:refund-2-line" width="16" />
					<span>Refunds</span>
				</a>
				<a href="/policies/disclaimer">
					<Icon icon="ri:alert-line" width="16" />
					<span>Disclaimer</span>
				</a>
			</nav>

			<div class="social-links">
				<a href="https://twitter.com/clickswave" target="_blank" aria-label="Twitter">
					<Icon icon="ri:twitter-x-line" width="20" />
				</a>
				<a href="https://github.com/clickswave" target="_blank" aria-label="GitHub">
					<Icon icon="ri:github-fill" width="20" />
				</a>
				<a href="https://discord.gg/clickswave" target="_blank" aria-label="Discord">
					<Icon icon="ri:discord-fill" width="20" />
				</a>
				<a href="https://linkedin.com/company/clickswave" target="_blank" aria-label="LinkedIn">
					<Icon icon="ri:linkedin-fill" width="20" />
				</a>
			</div>
		</div>
	</div>

	<Version version="0.0.1" />
</footer>


<style lang="scss">
	.landing-footer {
		padding: 4rem 2rem;
		background: rgba(11, 11, 13, 0.8);
		backdrop-filter: blur(20px);
		border-top: 1px solid rgba(255, 255, 255, 0.05);
		z-index: 10;
		position: relative;
		margin-top: auto;

		.footer-content {
			max-width: 1400px;
			margin: 0 auto;
			display: flex;
			justify-content: space-between;
			align-items: flex-end;
			gap: 4rem;

			@media (max-width: 900px) {
				flex-direction: column;
				align-items: flex-start;
				gap: 3rem;
			}
		}

		.brand-section {
			display: flex;
			flex-direction: column;
			gap: 1rem;

			.administered-by {
				color: #a1a1aa;
				font-size: 0.9rem;
				display: flex;
				flex-direction: column;
				gap: 0.5rem;

				span {
					text-transform: uppercase;
					font-size: 0.7rem;
					letter-spacing: 0.1em;
					opacity: 0.7;
				}

				.brand-name {
					display: flex;
					align-items: center;
					gap: 0.75rem;

					.cw-logo {
						height: 32px;
						width: auto;
						opacity: 0.9;
						transition: opacity 0.2s;
					}

					strong {
						color: white;
						font-family: 'Raleway', sans-serif;
						font-weight: 800;
						letter-spacing: 1px;
						font-size: 1.25rem;
					}

					&:hover .cw-logo {
						opacity: 1;
					}
				}
			}

			.copyright {
				color: #52525b;
				font-size: 0.85rem;
				margin: 0;

				.highlight {
					color: var(--primary, #ff4655);
					font-weight: 500;
				}
			}
		}

		.footer-right {
			display: flex;
			flex-direction: column;
			align-items: flex-end;
			gap: 1.5rem;

			@media (max-width: 900px) {
				align-items: flex-start;
				width: 100%;
			}

			.legal-nav {
				display: flex;
				gap: 1.5rem;
				flex-wrap: wrap;
				align-items: center;

				a {
					color: #71717a;
					text-decoration: none;
					font-size: 0.85rem;
					display: flex;
					align-items: center;
					gap: 0.4rem;
					transition: all 0.2s;
					padding: 0.4rem 0.8rem;
					border-radius: 6px;
					background: transparent;

					&:hover {
						color: white;
						background: rgba(255, 255, 255, 0.05);
					}
				}
			}

			.social-links {
				display: flex;
				gap: 1rem;

				a {
					color: #71717a;
					background: rgba(255, 255, 255, 0.03);
					width: 36px;
					height: 36px;
					border-radius: 50%;
					display: flex;
					align-items: center;
					justify-content: center;
					transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
					border: 1px solid rgba(255, 255, 255, 0.05);

					&:hover {
						color: white;
						background: var(--primary, #ff4655);
						border-color: var(--primary, #ff4655);
						transform: translateY(-3px);
						box-shadow: 0 4px 12px rgba(255, 70, 85, 0.3);
					}
				}
			}
		}
	}
</style>
