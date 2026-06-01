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
		<div class="hero">
			<h1>We are <span class="text-gradient">SiloCat</span></h1>
			<p class="subtitle">Redefining secure, anonymous file sharing for the modern web.</p>
		</div>

		<section class="mission">
			<div class="text-block">
				<h2>Built for the <span style="color: var(--primary, #ff4655);">Digital Feline</span></h2>
				<p>
					SiloCat operates in the shadows. We are built for those who value agility, silence, and
					absolute control. Whether you're a developer moving artifacts or a researcher sharing
					samples, our infrastructure is designed to be as elusive as a cat in the night.
				</p>
				<p>
					We operate under the vigilant eye of <strong style="color: white;">WatchCat</strong>, our
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
	</main>

	<Footer />

	<div class="bg-effects">
		<div class="glow-spot top"></div>
	</div>
</div>

<style lang="scss">
	:global(body) {
		margin: 0;
		background-color: #0b0b0d;
		color: white;
		font-family: 'Outfit', sans-serif;
	}

	.page-container {
		min-height: 100vh;
		position: relative;
		overflow-x: hidden;
		display: flex;
		flex-direction: column;
	}

	.content {
		padding: 4rem 2rem;
		position: relative;
		z-index: 10;
		max-width: 1000px;
		margin: 0 auto;
		flex: 1;
	}

	.hero {
		text-align: center;
		margin-bottom: 6rem;

		h1 {
			font-size: 4rem;
			font-weight: 800;
			margin-bottom: 1rem;

			.text-gradient {
				background: linear-gradient(135deg, #fff 0%, #a1a1aa 100%);
				-webkit-background-clip: text;
				-webkit-text-fill-color: transparent;
			}
		}

		.subtitle {
			font-size: 1.5rem;
			color: #a1a1aa;
			max-width: 600px;
			margin: 0 auto;
		}
	}

	.mission {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 4rem;
		align-items: center;

		@media (max-width: 768px) {
			grid-template-columns: 1fr;
			gap: 3rem;
		}

		.text-block {
			h2 {
				font-size: 2rem;
				margin-bottom: 1.5rem;
				color: var(--primary, #ff4655);
			}
			p {
				color: #d4d4d8;
				line-height: 1.7;
				margin-bottom: 1.5rem;
				font-size: 1.1rem;
			}
		}

		.stats-grid {
			display: grid;
			grid-template-columns: 1fr;
			gap: 2rem;

			.stat {
				background: rgba(255, 255, 255, 0.03);
				border: 1px solid rgba(255, 255, 255, 0.1);
				padding: 2rem;
				border-radius: 16px;
				text-align: center;
				transition: transform 0.2s;

				&:hover {
					transform: translateY(-5px);
					background: rgba(255, 255, 255, 0.05);
				}

				.value {
					display: block;
					font-size: 2.5rem;
					font-weight: 800;
					margin-bottom: 0.5rem;
					color: white;
				}

				.label {
					color: #a1a1aa;
					font-weight: 500;
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
