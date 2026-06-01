<script>
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { FrontendClient } from '$lib/frontendClient.js';
	import { toast } from 'svelte-sonner';

	onMount(async () => {
		const code = $page.url.searchParams.get('code');
		if (!code) {
			toast.error('No authorization code found');
			goto('/auth/signin');
			return;
		}

		try {
			const res = await FrontendClient.post('/api/v1/user/google-auth', { code });
			// Backend returns standard response structure { status, message, data }
			// Proxy returns that directly.
			if (res.data.status === 200) {
				toast.success('Logged in with Google!');
				goto('/home');
			} else {
				toast.error(res.data.message || 'Login failed');
				goto('/auth/signin');
			}
		} catch (e) {
			console.error(e);
			let msg = e.response?.data?.message || 'Google login failed';
			toast.error(msg);
			goto('/auth/signin');
		}
	});
</script>

<div class="callback-container">
	<div class="loader"></div>
	<p>Authenticating with Google...</p>
</div>

<style>
	.callback-container {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		height: 100vh;
		background-color: #09090b;
		color: #fff;
	}
	.loader {
		border: 4px solid #333;
		border-top: 4px solid #fff;
		border-radius: 50%;
		width: 40px;
		height: 40px;
		animation: spin 1s linear infinite;
		margin-bottom: 20px;
	}
	@keyframes spin {
		0% {
			transform: rotate(0deg);
		}
		100% {
			transform: rotate(360deg);
		}
	}
</style>
