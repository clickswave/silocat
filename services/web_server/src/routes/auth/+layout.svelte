<script>
	let { children } = $props();
	import Navbar from '$lib/components/Navbar.svelte';
	import Footer from '$lib/components/Footer.svelte';
</script>

<div class="auth-layout">
	<Navbar />

	<div class="content-wrapper">
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
	}

	.content-wrapper {
		flex: 1;
		display: flex;
		align-items: center;
		justify-content: center;
		padding-block: var(--space-10);
		padding-inline: var(--gutter);
	}

	.content {
		width: 100%;
		max-width: 400px;
		display: flex;
		flex-direction: column;
		align-items: center;
	}
</style>
