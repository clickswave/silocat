<script>
	import { enhance } from '$app/forms';
	import Seo from '$lib/components/Seo.svelte';
	import { goto } from '$app/navigation';
	import { toast } from 'svelte-sonner';
	
	import Icon from '$lib/ui/Icon.svelte';
	import { Turnstile } from 'svelte-turnstile';
	import { browser } from '$app/environment';
	import { slide } from 'svelte/transition';
	import { Button, Input } from '$lib/ui';

	let { data } = $props();

	let form = $state({
		email: '',
		username: '',
		password: '',
		confirmPassword: '',
		promoCode: ''
	});

	let showPromo = $state(false);

	let googleAuthUrl = $derived(
		browser
			? `https://accounts.google.com/o/oauth2/v2/auth?client_id=${data.googleClientId}&redirect_uri=${window.location.origin}/auth/callback&response_type=code&scope=openid%20email%20profile`
			: ''
	);

	let turnstileRef = $state(null);

	let loading = $state(false);
	function handleSubmit(event) {
		loading = true;
		return async ({ update, result }) => {
			if (result.data.success) {
				await goto('/home/pending-actions');
			}
			if (result.data.error) {
				let response = result.data.error;
				toast.error(response.message, {
					description: response.errors.join(', ')
				});
			}
			loading = false;
			turnstileRef?.reset?.();
			await update({ reset: false });
		};
	}
</script>

<Seo title="Create your account | Silocat" description="Create your encrypted Silocat account." noindex />

<section class="auth-card">
	<div class="card-header">
		<img class="mark" src="/silocat-logo.png" alt="" />
		<h1>Create account</h1>
		<p>10 GB free, encrypted end to end.</p>
	</div>

	<Button variant="ghost" block href={googleAuthUrl}>
		<Icon icon="logos:google-icon" width="16" /> Continue with Google
	</Button>

	<div class="or-divider"><span>or</span></div>

	<form method="POST" use:enhance={handleSubmit}>
		<Input
			bind:value={form.username}
			name="username"
			label="Username"
			icon="ri:user-line"
			placeholder="username"
			autocomplete="username"
			required
		/>

		<Input
			bind:value={form.email}
			name="email"
			type="email"
			label="Email"
			icon="ri:mail-line"
			placeholder="name@example.com"
			autocomplete="email"
			required
		/>

		<div class="grid-row">
			<Input
				bind:value={form.password}
				name="password"
				type="password"
				label="Password"
				icon="ri:lock-2-line"
				placeholder="••••••••"
				autocomplete="new-password"
				required
			/>

			<Input
				bind:value={form.confirmPassword}
				name="confirmPassword"
				type="password"
				label="Confirm"
				icon="ri:lock-2-line"
				placeholder="••••••••"
				autocomplete="new-password"
				required
			/>
		</div>

		{#if showPromo}
			<div transition:slide={{ duration: 150 }}>
				<Input
					bind:value={form.promoCode}
					name="promoCode"
					label="Promo code"
					icon="ri:ticket-line"
					placeholder="Code"
				/>
			</div>
		{:else}
			<button type="button" class="link-btn promo-toggle" onclick={() => (showPromo = true)}>
				Have a promo code?
			</button>
			<input type="hidden" name="promoCode" value={form.promoCode} />
		{/if}

		<div class="turnstile-group">
			<Turnstile siteKey={data.turnstileSiteKey} bind:this={turnstileRef} theme="auto" />
		</div>

		<Button type="submit" block {loading}>Create account</Button>
	</form>

	<div class="footer">
		<p>Already have an account? <a href="/auth/signin">Sign in</a></p>
	</div>
</section>

<style lang="scss">
	.auth-card {
		width: 100%;
		background: var(--surface);
		border: 1px solid var(--edge);
		border-radius: var(--radius-lg);
		padding: var(--space-6);
		display: flex;
		flex-direction: column;
		gap: var(--space-5);
	}

	.card-header {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
		align-items: center;
		text-align: center;

		.mark {
			width: 32px;
			height: 32px;
			margin-bottom: var(--space-1);
		}

		h1 {
			font-size: var(--fs-h3);
			margin: 0;
		}

		p {
			color: var(--ink-mute);
			font-size: var(--fs-sm);
			margin: 0;
		}
	}

	.or-divider {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		color: var(--ink-faint);
		font-size: var(--fs-xs);
		text-transform: uppercase;
		letter-spacing: 0.12em;

		&::before,
		&::after {
			content: '';
			flex: 1;
			height: 1px;
			background: var(--edge);
		}
	}

	form {
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
	}

	.grid-row {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: var(--space-3);

		/* let grid children shrink below content width instead of overflowing */
		> :global(*) {
			min-width: 0;
		}

		@media (max-width: 560px) {
			grid-template-columns: 1fr;
		}
	}

	.link-btn {
		background: none;
		border: none;
		padding: 0;
		cursor: pointer;
		color: var(--ink-faint);
		font-size: var(--fs-sm);
		font-family: inherit;
		align-self: flex-start;

		&:hover {
			color: var(--ink);
			text-decoration: underline;
		}
	}

	.turnstile-group {
		display: flex;
		justify-content: center;
	}

	.footer {
		text-align: center;

		p {
			color: var(--ink-faint);
			font-size: var(--fs-sm);
			margin: 0;

			a {
				color: var(--ink);
				font-weight: var(--fw-medium);

				&:hover {
					text-decoration: underline;
				}
			}
		}
	}
</style>
