<script>
	import { enhance } from '$app/forms';
	import { toast } from 'svelte-sonner';
	import SiloCatLogo from '$lib/assets/silo-cat.png';
	import { Turnstile } from 'svelte-turnstile';
	import Icon from '@iconify/svelte';

	let { data } = $props();

	let form = $state({
		email: '',
		password: ''
	});

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

<svelte:head>
	<title>Sign In - SiloCat Sanctum Access</title>
	<meta
		name="description"
		content="Access your encrypted SiloCat vault. Secure, anonymous, and encrypted file management. Kitty powered E2E encryption."
	/>
	<meta property="og:title" content="Sign In - SiloCat Sanctum" />
	<meta
		property="og:description"
		content="Kitty powered E2E encrypted anonymous file-sharing and cloud storage platform with parallel downloads."
	/>
</svelte:head>

<section class="auth-card">
	<div class="card-header">
		<div class="logo">
			<img src={SiloCatLogo} alt="SiloCat Logo" />
		</div>
		<div class="title-group">
			<h1>Enter the Sanctum</h1>
			<p>Access your encrypted vault</p>
		</div>
	</div>

	<div class="divider"></div>

	<form method="POST" use:enhance={handleSubmit}>
		<div class="form-group">
			<label for="email">Email</label>
			<div class="input-wrapper">
				<Icon icon="ri:mail-line" class="input-icon" width="18" />
				<input
					id="email"
					name="email"
					type="email"
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
					placeholder="••••••••"
					bind:value={form.password}
					autocomplete="current-password"
					required
				/>
			</div>
		</div>

		<div class="form-group" style="align-items: center; margin-top: 0.5rem;">
			<Turnstile siteKey={data.turnstileSiteKey} bind:this={turnstileRef} theme="auto" />
		</div>

		<button type="submit" disabled={loading} class="submit-btn">
			{#if loading}
				<Icon icon="line-md:loading-loop" width="20" /> Authenticating...
			{:else}
				<Icon icon="ri:login-circle-line" width="20" /> Access Vault
			{/if}
		</button>
	</form>

	<div class="footer">
		<p>New here? <a href="/auth/signup">Join the Watch</a></p>
	</div>

	<div class="divider"></div>

	<a
		href={`https://accounts.google.com/o/oauth2/v2/auth?client_id=${data.googleClientId}&redirect_uri=http://localhost:5173/auth/callback&response_type=code&scope=email%20profile`}
		class="google-btn"
	>
		<Icon icon="logos:google-icon" width="20" /> Continue with Google
	</a>
	<br />
</section>

<style lang="scss">
	.google-btn {
		background: white;
		color: #333;
		border: none;
		padding: 0.8rem;
		border-radius: 12px;
		font-weight: 600;
		font-size: 1rem;
		cursor: pointer;
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 0.5rem;
		text-decoration: none;
		margin-top: 1rem;
		transition: transform 0.2s;

		&:hover {
			transform: translateY(-2px);
			background: #f1f1f1;
		}
	}

	.auth-card {
		background: rgba(20, 20, 22, 0.6);
		backdrop-filter: blur(24px);
		border: 1px solid rgba(255, 255, 255, 0.08);
		padding: 2.5rem;
		border-radius: 24px;
		width: 100%;
		box-shadow: 0 40px 80px rgba(0, 0, 0, 0.4);
		display: flex;
		flex-direction: column;
		gap: 2rem;
		position: relative;
		overflow: hidden;

		/* Top glowing edge */
		&::before {
			content: '';
			position: absolute;
			top: 0;
			left: 0;
			right: 0;
			height: 1px;
			background: linear-gradient(90deg, transparent, rgba(255, 70, 85, 0.5), transparent);
		}
	}

	.card-header {
		display: flex;
		flex-direction: column;
		gap: 1.5rem;
		align-items: center;

		.logo {
			width: 72px;
			height: 72px;
			background: radial-gradient(circle at center, rgba(255, 255, 255, 0.05), transparent);
			border: 1px solid rgba(255, 255, 255, 0.1);
			border-radius: 20px;
			display: flex;
			align-items: center;
			justify-content: center;
			box-shadow: 0 0 30px rgba(0, 0, 0, 0.2);

			img {
				width: 36px;
				height: 36px;
				filter: drop-shadow(0 0 10px rgba(255, 255, 255, 0.1));
			}
		}

		.title-group {
			text-align: center;

			h1 {
				margin: 0 0 0.5rem 0;
				font-size: 1.75rem;
				font-weight: 700;
				color: white;
				letter-spacing: -0.02em;
			}

			p {
				margin: 0;
				color: #a1a1aa;
				font-size: 1rem;
			}
		}
	}

	.divider {
		height: 1px;
		background: rgba(255, 255, 255, 0.05);
		width: 100%;
	}

	form {
		display: flex;
		flex-direction: column;
		gap: 1.25rem;
	}

	.form-group {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;

		label {
			font-size: 0.9rem;
			font-weight: 500;
			color: #d4d4d8;
			margin-left: 2px;
		}

		.input-wrapper {
			position: relative;
			display: flex;
			align-items: center;

			:global(.input-icon) {
				position: absolute;
				left: 1rem;
				color: #71717a;
				pointer-events: none;
				transition: color 0.2s;
			}

			&:focus-within :global(.input-icon) {
				color: var(--primary, #ff4655);
			}
		}

		input {
			background: rgba(0, 0, 0, 0.2);
			border: 1px solid rgba(255, 255, 255, 0.08);
			padding: 1rem 1rem 1rem 2.75rem;
			border-radius: 12px;
			color: white;
			font-size: 1rem;
			outline: none;
			transition: all 0.2s;
			width: 100%;

			&:focus {
				border-color: var(--primary, #ff4655);
				background: rgba(255, 70, 85, 0.05);
				box-shadow: 0 0 0 1px var(--primary, #ff4655);
			}

			&::placeholder {
				color: #3f3f46;
			}
		}
	}

	.submit-btn {
		background: var(--primary, #ff4655);
		border: none;
		padding: 1rem;
		border-radius: 12px;
		color: white;
		font-weight: 600;
		font-size: 1rem;
		cursor: pointer;
		transition: all 0.2s;
		box-shadow: 0 4px 20px rgba(255, 70, 85, 0.3);
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 0.5rem;
		margin-top: 0.5rem;

		&:hover {
			background: #e03e4b;
			transform: translateY(-2px);
			box-shadow: 0 8px 25px rgba(255, 70, 85, 0.4);
		}

		&:active {
			transform: translateY(0);
		}

		&:disabled {
			opacity: 0.7;
			cursor: not-allowed;
			transform: none;
		}
	}

	.footer {
		text-align: center;

		p {
			color: #71717a;
			font-size: 0.9rem;
			margin: 0;

			a {
				color: white;
				text-decoration: none;
				font-weight: 500;
				transition: color 0.2s;

				&:hover {
					color: var(--primary, #ff4655);
					text-decoration: underline;
				}
			}
		}
	}
</style>
