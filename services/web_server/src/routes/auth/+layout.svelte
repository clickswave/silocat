<script>
	let { children } = $props();
	import Navbar from '$lib/components/Navbar.svelte';
	import Footer from '$lib/components/Footer.svelte';
</script>

<div class="auth-layout">
	<Navbar />

	<div class="content-wrapper">
		<div class="background-effects">
			<div class="stars"></div>
			<div class="glow-spot"></div>
		</div>

		<div class="content">
			{@render children()}
		</div>
	</div>

	<Footer />
</div>

<style lang="scss">
	:global(body) {
		margin: 0;
		background-color: #0b0b0d;
		color: white;
		font-family: 'Outfit', sans-serif;
	}

	.auth-layout {
		min-height: 100vh;
		display: flex;
		flex-direction: column;
		background: #0b0b0d;
	}

	.content-wrapper {
		flex: 1;
		display: flex;
		align-items: center;
		justify-content: center;
		position: relative;
		overflow: hidden;
		padding: 2rem; /* Ensure space on mobile */
	}

	.background-effects {
		position: absolute;
		inset: 0;
		z-index: 0;
		pointer-events: none;

		.glow-spot {
			position: absolute;
			top: 50%;
			left: 50%;
			transform: translate(-50%, -50%);
			width: 800px;
			height: 800px;
			background: radial-gradient(circle, rgba(255, 70, 85, 0.05) 0%, transparent 70%);
			filter: blur(80px);
		}

		.stars {
			position: absolute;
			inset: 0;
			background-image:
				radial-gradient(1px 1px at 20px 30px, #ffffff, rgba(0, 0, 0, 0)),
				radial-gradient(1px 1px at 40px 70px, #ffffff, rgba(0, 0, 0, 0)),
				radial-gradient(1px 1px at 50px 160px, #ffffff, rgba(0, 0, 0, 0));
			background-repeat: repeat;
			background-size: 200px 200px;
			opacity: 0.1;
		}
	}

	.content {
		position: relative;
		z-index: 1;
		width: 100%;
		max-width: 440px;
		display: flex;
		flex-direction: column;
		align-items: center;
	}
</style>
