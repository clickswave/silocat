<script>
	import { enhance } from '$app/forms';
	import Seo from '$lib/components/Seo.svelte';
	import { toast } from 'svelte-sonner';
	import SiloCatLogo from '$lib/assets/silo-cat.png';
	import { Turnstile } from 'svelte-turnstile';
	import Icon from '@iconify/svelte';
	import { browser } from '$app/environment';
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';

	let { data } = $props();

	onMount(() => {
		if (browser && new URLSearchParams(window.location.search).get('banned')) {
			toast.error('Your account has been banned. Contact support if you believe this is a mistake.');
		}
	});

	let form = $state({
		email: '',
		password: ''
	});

	// 'login' | 'forgot' (enter email) | 'reset' (enter code + new password)
	let mode = $state('login');
	let resetEmail = $state('');
	let resetOtp = $state('');
	let resetNewPassword = $state('');
	let resetBusy = $state(false);

	function openForgot() {
		resetEmail = form.email || '';
		resetOtp = '';
		resetNewPassword = '';
		mode = 'forgot';
	}

	async function sendResetCode(e) {
		e?.preventDefault?.();
		if (!resetEmail.trim()) {
			toast.error('Enter your email first');
			return;
		}
		resetBusy = true;
		try {
			const res = await fetch('/api/v1/user/forgot-password', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({ email: resetEmail.trim() })
			});
			const d = await res.json();
			if (!res.ok) {
				toast.error(d.error || 'Could not send the code');
			} else {
				toast.success('If an account exists, a reset code is on its way.');
				mode = 'reset';
			}
		} catch {
			toast.error('Network error, please try again');
		} finally {
			resetBusy = false;
		}
	}

	async function submitReset(e) {
		e?.preventDefault?.();
		if (!resetOtp.trim() || !resetNewPassword) {
			toast.error('Enter the code and a new password');
			return;
		}
		resetBusy = true;
		try {
			const res = await fetch('/api/v1/user/reset-password', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					email: resetEmail.trim(),
					otp: resetOtp.trim(),
					new_password: resetNewPassword
				})
			});
			const d = await res.json();
			if (!res.ok) {
				toast.error(d.error || 'Could not reset your password');
			} else {
				toast.success('Password reset. Welcome back!');
				await goto('/home');
			}
		} catch {
			toast.error('Network error, please try again');
		} finally {
			resetBusy = false;
		}
	}

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
			// if (result.data.success) await goto('/dashboard');
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
	title="Sign in | SiloCat"
	description="Access your encrypted SiloCat vault."
	noindex
/>

<section class="auth-card card">
	<div class="card-header">
		<div class="logo">
			<img src={SiloCatLogo} alt="SiloCat Logo" />
		</div>
		<div class="title-group">
			{#if mode === 'login'}
				<h1>Enter the Sanctum</h1>
				<p>Access your encrypted vault.</p>
			{:else if mode === 'forgot'}
				<h1>Reset password</h1>
				<p>We'll email you a one-time code.</p>
			{:else}
				<h1>Choose a new password</h1>
				<p>Enter the code we emailed and your new password.</p>
			{/if}
		</div>
	</div>

	{#if mode === 'login'}
	<form method="POST" use:enhance={handleSubmit}>
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

		<div class="form-group">
			<label for="password">Password</label>
			<div class="input-wrapper">
				<Icon icon="ri:lock-password-line" class="input-icon" width="18" />
				<input
					id="password"
					name="password"
					type="password"
					class="field"
					placeholder="••••••••"
					bind:value={form.password}
					autocomplete="current-password"
					required
				/>
			</div>
			<button type="button" class="link-btn forgot-link" onclick={openForgot}>
				Forgot password?
			</button>
		</div>

		<div class="form-group turnstile-group">
			<Turnstile siteKey={data.turnstileSiteKey} bind:this={turnstileRef} theme="auto" />
		</div>

		<button type="submit" disabled={loading} class="btn btn-primary btn-block submit-btn">
			{#if loading}
				<Icon icon="line-md:loading-loop" width="20" /> Authenticating...
			{:else}
				<Icon icon="ri:login-circle-line" width="20" /> Access Vault
			{/if}
		</button>
	</form>

	<div class="or-divider"><span>or</span></div>

	<a href={googleAuthUrl} class="btn btn-ghost btn-block google-btn">
		<Icon icon="logos:google-icon" width="20" /> Continue with Google
	</a>

	<div class="footer">
		<p>New here? <a href="/auth/signup">Join the Watch</a></p>
	</div>

	{:else if mode === 'forgot'}
	<form onsubmit={sendResetCode}>
		<div class="form-group">
			<label for="reset-email">Email</label>
			<div class="input-wrapper">
				<Icon icon="ri:mail-line" class="input-icon" width="18" />
				<input
					id="reset-email"
					type="email"
					class="field"
					placeholder="name@example.com"
					bind:value={resetEmail}
					autocomplete="email"
					required
				/>
			</div>
		</div>

		<button type="submit" disabled={resetBusy} class="btn btn-primary btn-block submit-btn">
			{#if resetBusy}
				<Icon icon="line-md:loading-loop" width="20" /> Sending...
			{:else}
				<Icon icon="ri:mail-send-line" width="20" /> Send reset code
			{/if}
		</button>
	</form>

	<div class="footer">
		<p><button type="button" class="link-btn" onclick={() => (mode = 'login')}>Back to sign in</button></p>
	</div>

	{:else}
	<form onsubmit={submitReset}>
		<div class="form-group">
			<label for="reset-otp">Reset code</label>
			<div class="input-wrapper">
				<Icon icon="ri:shield-keyhole-line" class="input-icon" width="18" />
				<input
					id="reset-otp"
					type="text"
					inputmode="numeric"
					class="field"
					placeholder="6-digit code"
					bind:value={resetOtp}
					autocomplete="one-time-code"
					required
				/>
			</div>
		</div>

		<div class="form-group">
			<label for="reset-new-password">New password</label>
			<div class="input-wrapper">
				<Icon icon="ri:lock-password-line" class="input-icon" width="18" />
				<input
					id="reset-new-password"
					type="password"
					class="field"
					placeholder="••••••••"
					bind:value={resetNewPassword}
					autocomplete="new-password"
					required
				/>
			</div>
		</div>

		<button type="submit" disabled={resetBusy} class="btn btn-primary btn-block submit-btn">
			{#if resetBusy}
				<Icon icon="line-md:loading-loop" width="20" /> Resetting...
			{:else}
				<Icon icon="ri:lock-unlock-line" width="20" /> Reset password & sign in
			{/if}
		</button>
	</form>

	<div class="footer">
		<p>
			<button type="button" class="link-btn" onclick={sendResetCode} disabled={resetBusy}>Resend code</button>
			<span class="sep">·</span>
			<button type="button" class="link-btn" onclick={() => (mode = 'login')}>Back to sign in</button>
		</p>
	</div>
	{/if}
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

	.form-group {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);

		label {
			font-size: var(--fs-sm);
			font-weight: var(--fw-medium);
			color: var(--text-secondary);
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

	.link-btn {
		background: none;
		border: none;
		padding: 0;
		cursor: pointer;
		color: var(--text-primary);
		font-size: var(--fs-sm);
		font-weight: var(--fw-medium);
		font-family: inherit;

		&:hover {
			color: var(--primary);
			text-decoration: underline;
		}

		&:disabled {
			opacity: 0.6;
			cursor: default;
		}
	}

	.forgot-link {
		align-self: flex-end;
		margin-top: calc(-1 * var(--space-1));
		font-size: var(--fs-xs);
		color: var(--text-muted);
	}

	.footer .sep {
		color: var(--text-muted);
		margin: 0 var(--space-2);
	}

	.google-btn {
		text-decoration: none;
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
</style>
