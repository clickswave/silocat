<script>
	import { onMount } from 'svelte';

	onMount(() => {
		window.location.href = '/login';
	});
</script>

<div class="redirect-msg">Redirecting to login...</div>

<style>
	.redirect-msg {
		height: 100vh;
		display: flex;
		justify-content: center;
		align-items: center;
		color: #666;
	}
</style>
