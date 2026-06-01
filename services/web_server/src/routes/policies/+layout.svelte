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
			<h3>Legal Center</h3>
			<div class="links">
				{#each links as link}
					<a href={link.href} class:active={$page.url.pathname === link.href}>
						{link.label}
					</a>
				{/each}
			</div>
		</aside>

		<main class="main-content">
			<slot />
		</main>
	</div>

	<Footer />
</div>

<style lang="scss">
	:global(html),
	:global(body) {
		margin: 0;
		background-color: #0b0b0d;
		color: white;
		font-family: 'Outfit', sans-serif;
	}

	.policy-layout {
		min-height: 100vh;
		display: flex;
		flex-direction: column;
	}

	.content-wrapper {
		display: flex;
		flex: 1;
		max-width: 1400px;
		margin: 0 auto;
		width: 100%;
		position: relative;
		z-index: 10;

		@media (max-width: 768px) {
			flex-direction: column;
		}
	}

	.sidebar {
		width: 300px;
		padding: 3rem 2rem;
		border-right: 1px solid rgba(255, 255, 255, 0.05);

		@media (max-width: 768px) {
			width: 100%;
			border-right: none;
			border-bottom: 1px solid rgba(255, 255, 255, 0.05);
			padding: 1.5rem;
		}

		h3 {
			color: #71717a;
			font-size: 0.85rem;
			text-transform: uppercase;
			letter-spacing: 0.05em;
			margin-bottom: 1.5rem;
		}

		.links {
			display: flex;
			flex-direction: column;
			gap: 0.5rem;

			a {
				color: #a1a1aa;
				text-decoration: none;
				padding: 0.75rem 1rem;
				border-radius: 8px;
				transition: all 0.2s;
				font-weight: 500;

				&:hover {
					color: white;
					background: rgba(255, 255, 255, 0.03);
				}

				&.active {
					background: rgba(255, 70, 85, 0.1);
					color: var(--primary, #ff4655);
				}
			}
		}
	}

	.main-content {
		flex: 1;
		padding: 4rem 4rem;
		max-width: 900px;

		@media (max-width: 768px) {
			padding: 2rem;
		}
	}
</style>
