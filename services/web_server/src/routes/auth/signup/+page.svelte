<script>
	import { enhance } from '$app/forms';
	import Seo from '$lib/components/Seo.svelte';
	import { goto } from '$app/navigation';
	import { toast } from 'svelte-sonner';
	import SiloCatLogo from '$lib/assets/silo-cat.png';
	import Icon from '@iconify/svelte';
	import { Turnstile } from 'svelte-turnstile';
	import { browser } from '$app/environment';

	let { data } = $props();

	let form = $state({
		email: '',
		username: '',
		password: '',
		confirmPassword: '',
		promoCode: ''
	});

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
			// on success
			if (result.data.success) {
				await goto('/home/pending-actions');
			}
			// on error
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

<Seo
	title="Create your account | SiloCat"
	description="Create your anonymous, encrypted SiloCat vault."
	noindex
/>

<section class="auth-card card">
	<div class="card-header">
		<div class="logo">
			<img src={SiloCatLogo} alt="SiloCat Logo" />
		</div>
		<div class="title-group">
			<h1>Create your encrypted vault</h1>
			<p>Zero-knowledge by design. Only you hold the keys.</p>
		</div>
	</div>

	<a href={googleAuthUrl} class="btn btn-ghost btn-block google-btn">
		<Icon icon="logos:google-icon" width="20" /> Continue with Google
	</a>

	<div class="or-divider"><span>or</span></div>

	<form method="POST" use:enhance={handleSubmit}>
		<div class="grid-row">
			<div class="form-group">
				<label for="username">Username</label>
				<div class="input-wrapper">
					<Icon icon="ri:user-line" class="input-icon" width="18" />
					<input
						id="username"
						name="username"
						type="text"
						class="field"
						placeholder="codename"
						bind:value={form.username}
						autocomplete="username"
						required
					/>
				</div>
			</div>

			<div class="form-group">
				<label for="email">Email</label>
				<div class="input-wrapper">
					<Icon icon="ri:mail-line" class="input-icon" width="18" />
					<input
						id="email"
						name="email"
						type="email"
						class="field"
						placeholder="name@example.com"
						bind:value={form.email}
						autocomplete="email"
						required
					/>
				</div>
			</div>
		</div>

		<div class="grid-row">
			<div class="form-group">
				<label for="password">Password</label>
				<div class="input-wrapper">
					<Icon icon="ri:lock-password-line" class="input-icon" width="18" />
					<input
						id="password"
						name="password"
						type="password"
						class="field"
						placeholder="••••••"
						bind:value={form.password}
						autocomplete="new-password"
						required
					/>
				</div>
			</div>

			<div class="form-group">
				<label for="confirmPassword">Confirm</label>
				<div class="input-wrapper">
					<Icon icon="ri:lock-check-line" class="input-icon" width="18" />
					<input
						id="confirmPassword"
						name="confirmPassword"
						type="password"
						class="field"
						placeholder="••••••"
						bind:value={form.confirmPassword}
						autocomplete="new-password"
						required
					/>
				</div>
			</div>
		</div>

		<div class="form-group">
			<label for="promoCode">
				Promo code
				<span class="optional">Have a promo code? Unlock bonus storage.</span>
			</label>
			<div class="input-wrapper">
				<Icon icon="ri:ticket-line" class="input-icon" width="18" />
				<input
					id="promoCode"
					name="promoCode"
					type="text"
					class="field"
					placeholder="Promo code (optional)"
					bind:value={form.promoCode}
				/>
			</div>
		</div>

		<div class="form-group turnstile-group">
			<Turnstile siteKey={data.turnstileSiteKey} bind:this={turnstileRef} theme="auto" />
		</div>

		<button type="submit" disabled={loading} class="btn btn-primary btn-block submit-btn">
			{#if loading}
				<Icon icon="line-md:loading-loop" width="20" /> Initializing...
			{:else}
				<Icon icon="ri:shield-user-line" width="20" /> Create Encrypted Vault
			{/if}
		</button>
	</form>

	<div class="footer">
		<p>Already an agent? <a href="/auth/signin">Access Terminal</a></p>
	</div>
</section>

<style lang="scss">
	.auth-card {
		width: 100%;
		max-width: 480px;
		padding: var(--space-6);
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
		position: relative;
		overflow: hidden;

		/* Top accent edge */
		&::before {
			content: '';
			position: absolute;
			top: 0;
			left: 0;
			right: 0;
			height: 1px;
			background: linear-gradient(90deg, transparent, var(--primary), transparent);
		}
	}

	.card-header {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		align-items: center;

		.logo {
			width: 48px;
			height: 48px;
			background: var(--tint-soft);
			border: 1px solid var(--border-default);
			border-radius: var(--radius-md);
			display: flex;
			align-items: center;
			justify-content: center;
			box-shadow: var(--shadow-glow);

			img {
				width: 26px;
				height: 26px;
			}
		}

		.title-group {
			text-align: center;
			display: flex;
			flex-direction: column;
			align-items: center;
			gap: var(--space-1);

			h1 {
				margin: 0;
				font-size: var(--fs-h3);
				font-weight: var(--fw-bold);
			}

			p {
				margin: 0;
				color: var(--text-secondary);
				font-size: var(--fs-sm);
			}
		}
	}

	.or-divider {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		color: var(--text-muted);
		font-size: var(--fs-xs);
		text-transform: uppercase;
		letter-spacing: 0.12em;

		&::before,
		&::after {
			content: '';
			flex: 1;
			height: 1px;
			background: var(--hairline);
		}
	}

	form {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
	}

	.grid-row {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: var(--space-3);
	}

	.form-group {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);

		label {
			font-size: var(--fs-sm);
			font-weight: var(--fw-medium);
			color: var(--text-secondary);

			.optional {
				font-size: var(--fs-xs);
				color: var(--text-muted);
				font-weight: var(--fw-regular);
			}
		}

		.input-wrapper {
			position: relative;
			display: flex;
			align-items: center;
			width: 100%;

			:global(.input-icon) {
				position: absolute;
				left: var(--space-3);
				color: var(--text-muted);
				pointer-events: none;
				transition: color var(--dur) var(--ease);
			}

			&:focus-within :global(.input-icon) {
				color: var(--primary);
			}
		}

		.field {
			padding-left: 2.6rem;
		}
	}

	.turnstile-group {
		align-items: center;
		margin-top: var(--space-1);
	}

	.submit-btn {
		margin-top: var(--space-1);
	}

	.footer {
		text-align: center;

		p {
			color: var(--text-muted);
			font-size: var(--fs-sm);
			margin: 0;

			a {
				color: var(--text-primary);
				text-decoration: none;
				font-weight: var(--fw-medium);
				transition: color var(--dur) var(--ease);

				&:hover {
					color: var(--primary);
					text-decoration: underline;
				}
			}
		}
	}

	@media (max-width: 560px) {
		.grid-row {
			grid-template-columns: 1fr;
		}
	}
</style>
