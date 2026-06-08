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
			const res = await FrontendClient.post('/api/v1/user/google-auth', {
				code,
				redirect_uri: `${window.location.origin}/auth/callback`
			});
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
		min-height: 60vh;
		gap: var(--space-4);
		color: var(--text-secondary);
		font-family: var(--font-sans);
		font-size: var(--fs-body);
	}
	.callback-container p {
		margin: 0;
	}
	.loader {
		border: 3px solid var(--border-strong);
		border-top-color: var(--primary);
		border-radius: 50%;
		width: 40px;
		height: 40px;
		animation: spin 1s linear infinite;
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
