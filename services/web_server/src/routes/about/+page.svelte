<script>
	import Navbar from '$lib/components/Navbar.svelte';
	import Footer from '$lib/components/Footer.svelte';
</script>

<svelte:head>
	<title>About SiloCat - Cat powered file sharing and storage platform</title>
	<meta
		name="description"
		content="Learn about SiloCat's mission to provide the most secure file sharing platform. Kitty powered E2E encrypted anonymous file-sharing."
	/>
	<meta property="og:title" content="About SiloCat - Kitty Powered Secure Storage" />
	<meta
		property="og:description"
		content="Kitty powered E2E encrypted anonymous file-sharing and cloud storage platform with parallel downloads."
	/>
</svelte:head>

<div class="page-container">
	<Navbar />

	<main class="content">
		<section class="section">
			<div class="container">
				<div class="hero">
					<span class="eyebrow">about silocat</span>
					<h1>We are <span class="text-gradient">SiloCat</span></h1>
					<p class="subtitle">Redefining secure, anonymous file sharing for the modern web.</p>
				</div>

				<section class="mission">
					<div class="text-block">
						<h2>Built for the <span class="accent">Digital Feline</span></h2>
						<p>
							SiloCat operates in the shadows. We are built for those who value agility, silence, and
							absolute control. Whether you're a developer moving artifacts or a researcher sharing
							samples, our infrastructure is designed to be as elusive as a cat in the night.
						</p>
						<p>
							We operate under the vigilant eye of <strong>WatchCat</strong>, our
							internal storage sentinel. WatchCat ensures your data lands in the optimal encrypted vault
							(R2 or B2) while remaining completely invisible to the outside world.
						</p>
						<p>
							However, we are not lawless. We strictly adhere to DMCA and government compliance
							regulations. We protect your privacy against prying eyes, not against the law. If you
							abuse the platform, the cat bites back.
						</p>
					</div>
			<div class="stats-grid">
				<div class="stat">
					<span class="value">Zero</span>
					<span class="label">Knowledge</span>
				</div>
				<div class="stat">
					<span class="value">WatchCat</span>
					<span class="label">Sentinel</span>
				</div>
				<div class="stat">
						<span class="value">R2 / B2</span>
						<span class="label">Multi-Cloud</span>
					</div>
				</div>
			</section>
			</div>
		</section>
	</main>

	<Footer />

	<div class="bg-effects">
		<div class="glow-spot top"></div>
	</div>
</div>

<style lang="scss">
	.page-container {
		min-height: 100vh;
		position: relative;
		overflow-x: hidden;
		display: flex;
		flex-direction: column;
	}

	.content {
		position: relative;
		z-index: 10;
		flex: 1;
	}

	.hero {
		text-align: center;
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: var(--space-4);
		margin-bottom: var(--space-12);

		h1 {
			font-size: var(--fs-display);
			font-weight: var(--fw-black);
		}

		.subtitle {
			font-size: var(--fs-lg);
			color: var(--text-secondary);
			max-width: 600px;
			margin: 0 auto;
		}
	}

	.mission {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: var(--space-8);
		align-items: center;

		@media (max-width: 768px) {
			grid-template-columns: 1fr;
			gap: var(--space-6);
		}

		.text-block {
			h2 {
				font-size: var(--fs-h2);
				margin-bottom: var(--space-5);

				.accent {
					color: var(--primary);
				}
			}
			p {
				color: var(--text-secondary);
				line-height: var(--lh-normal);
				margin-bottom: var(--space-5);
				font-size: var(--fs-lg);

				strong {
					color: var(--text-primary);
				}
			}
		}

		.stats-grid {
			display: grid;
			grid-template-columns: 1fr;
			gap: var(--space-5);

			.stat {
				background: var(--bg-card);
				border: 1px solid var(--border-default);
				box-shadow: var(--shadow-card);
				padding: var(--space-6);
				border-radius: var(--radius-md);
				text-align: center;
				transition: transform var(--dur) var(--ease), border-color var(--dur) var(--ease);

				&:hover {
					transform: translateY(-2px);
					border-color: var(--border-strong);
				}

				.value {
					display: block;
					font-size: var(--fs-h2);
					font-weight: var(--fw-black);
					margin-bottom: var(--space-2);
					color: var(--text-primary);
				}

				.label {
					color: var(--text-secondary);
					font-weight: var(--fw-medium);
				}
			}
		}
	}

	.bg-effects {
		position: absolute;
		inset: 0;
		z-index: 0;
		pointer-events: none;

		.glow-spot {
			position: absolute;
			width: 800px;
			height: 800px;
			background: radial-gradient(circle, rgba(255, 70, 85, 0.08) 0%, transparent 70%);
			filter: blur(100px);

			&.top {
				top: -20%;
				right: -20%;
			}
		}
	}
</style>
