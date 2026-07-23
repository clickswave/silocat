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
	import { Button, Input, PasswordInput, OtpInput } from '$lib/ui';

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
				toast.success('Password reset. Welcome back.');
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

<Seo title="Sign in | Silocat" description="Access your encrypted Silocat storage." noindex />

<section class="auth-card">
	<div class="card-header">
		<img class="mark" src={SiloCatLogo} alt="" />
		{#if mode === 'login'}
			<h1>Sign in</h1>
		{:else if mode === 'forgot'}
			<h1>Reset password</h1>
			<p>We'll email you a one-time code.</p>
		{:else}
			<h1>Choose a new password</h1>
			<p>Enter the code we emailed and a new password.</p>
		{/if}
	</div>

	{#if mode === 'login'}
		<form method="POST" use:enhance={handleSubmit}>
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

			<div class="password-group">
				<Input
					bind:value={form.password}
					name="password"
					type="password"
					label="Password"
					icon="ri:lock-2-line"
					placeholder="••••••••"
					autocomplete="current-password"
					required
				/>
				<button type="button" class="link-btn forgot-link" onclick={openForgot}>
					Forgot password?
				</button>
			</div>

			<div class="turnstile-group">
				<Turnstile siteKey={data.turnstileSiteKey} bind:this={turnstileRef} theme="auto" />
			</div>

			<Button type="submit" block {loading}>Sign in</Button>
		</form>

		<div class="or-divider"><span>or</span></div>

		<Button variant="ghost" block href={googleAuthUrl}>
			<Icon icon="logos:google-icon" width="16" /> Continue with Google
		</Button>

		<div class="footer">
			<p>New here? <a href="/auth/signup">Create account</a></p>
		</div>
	{:else if mode === 'forgot'}
		<form onsubmit={sendResetCode}>
			<Input
				bind:value={resetEmail}
				type="email"
				label="Email"
				icon="ri:mail-line"
				placeholder="name@example.com"
				autocomplete="email"
				required
			/>

			<Button type="submit" block loading={resetBusy}>Send reset code</Button>
		</form>

		<div class="footer">
			<p><button type="button" class="link-btn" onclick={() => (mode = 'login')}>Back to sign in</button></p>
		</div>
	{:else}
		<form onsubmit={submitReset}>
			<div class="otp-group">
				<span class="otp-label">Reset code</span>
				<OtpInput bind:value={resetOtp} />
			</div>

			<PasswordInput
				bind:value={resetNewPassword}
				label="New password"
				autocomplete="new-password"
				required
			/>

			<Button type="submit" block loading={resetBusy}>Reset password</Button>
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
		gap: var(--space-2);
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

	form {
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
	}

	.password-group {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
	}

	.forgot-link {
		align-self: flex-end;
		font-size: var(--fs-xs);
		color: var(--ink-faint);
	}

	.otp-group {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);

		.otp-label {
			font-size: var(--fs-sm);
			font-weight: var(--fw-medium);
			color: var(--ink-mute);
		}
	}

	.turnstile-group {
		display: flex;
		justify-content: center;
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

	.link-btn {
		background: none;
		border: none;
		padding: 0;
		cursor: pointer;
		color: var(--ink-mute);
		font-size: var(--fs-sm);
		font-weight: var(--fw-medium);
		font-family: inherit;

		&:hover {
			color: var(--ink);
			text-decoration: underline;
		}

		&:disabled {
			opacity: 0.6;
			cursor: default;
		}
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

		.sep {
			margin: 0 var(--space-2);
		}
	}
</style>
