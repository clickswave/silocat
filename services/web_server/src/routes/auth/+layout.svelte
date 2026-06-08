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
	.auth-layout {
		min-height: 100vh;
		display: flex;
		flex-direction: column;
		background: var(--bg-app);
		color: var(--text-primary);
		font-family: var(--font-sans);
	}

	.content-wrapper {
		flex: 1;
		display: flex;
		align-items: center;
		justify-content: center;
		position: relative;
		overflow: hidden;
		padding-block: var(--space-10);
		padding-inline: var(--gutter);
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
			background: radial-gradient(circle, var(--primary-glow) 0%, transparent 70%);
			opacity: 0.18;
			filter: blur(110px);
		}

		.stars {
			position: absolute;
			inset: 0;
			background-image:
				radial-gradient(1px 1px at 20px 30px, var(--text-primary), transparent),
				radial-gradient(1px 1px at 40px 70px, var(--text-primary), transparent),
				radial-gradient(1px 1px at 50px 160px, var(--text-primary), transparent);
			background-repeat: repeat;
			background-size: 200px 200px;
			opacity: 0.08;
		}
	}

	.content {
		position: relative;
		z-index: 1;
		width: 100%;
		max-width: 480px;
		display: flex;
		flex-direction: column;
		align-items: center;
	}
</style>
