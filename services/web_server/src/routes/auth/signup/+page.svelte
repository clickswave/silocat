<script>
	import { enhance } from '$app/forms';
	import { goto } from '$app/navigation';
	import { toast } from 'svelte-sonner';
	import SiloCatLogo from '$lib/assets/silo-cat.png';
	import Icon from '@iconify/svelte';
	import { Turnstile } from 'svelte-turnstile';

	let { data } = $props();

	let form = $state({
		email: '',
		username: '',
		password: '',
		confirmPassword: '',
		inviteCode: ''
	});

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

<svelte:head>
	<title>Join the Watch - Create Secure Account | SiloCat</title>
	<meta
		name="description"
		content="Create your anonymous, encrypted vault. Join the SiloCat watch. Kitty powered E2E encrypted anonymous file-sharing."
	/>
	<meta property="og:title" content="Join the Watch - SiloCat" />
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
			<h1>Join the Watch</h1>
			<p>Secure your digital footprint</p>
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
			<label for="username">Username</label>
			<div class="input-wrapper">
				<Icon icon="ri:user-line" class="input-icon" width="18" />
				<input
					id="username"
					name="username"
					type="text"
					placeholder="codename"
					bind:value={form.username}
					autocomplete="username"
					required
				/>
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
						placeholder="••••••"
						bind:value={form.confirmPassword}
						autocomplete="new-password"
						required
					/>
				</div>
			</div>
		</div>

		<div class="form-group">
			<label for="inviteCode">
				Invite Code
				{#if data.inviteOnly}
					<span class="required">(Required)</span>
					<span class="request-link"> — <a href="/early-access">Request one</a></span>
				{:else}
					<span class="optional">(Optional)</span>
					<span class="request-link"> · unlocks bonus storage / Pro</span>
				{/if}
			</label>
			<div class="input-wrapper">
				<Icon icon="ri:ticket-line" class="input-icon" width="18" />
				<input
					id="inviteCode"
					name="inviteCode"
					type="text"
					placeholder="INV-XXXX"
					bind:value={form.inviteCode}
					required={data.inviteOnly}
				/>
			</div>
		</div>

		<div class="form-group" style="align-items: center; margin-top: 0.5rem;">
			<Turnstile siteKey={data.turnstileSiteKey} bind:this={turnstileRef} theme="auto" />
		</div>

		<button type="submit" disabled={loading} class="submit-btn">
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
		background: rgba(20, 20, 22, 0.6);
		backdrop-filter: blur(24px);
		border: 1px solid rgba(255, 255, 255, 0.08);
		padding: 2.5rem;
		border-radius: 24px;
		width: 100%;
		max-width: 460px;
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

	.grid-row {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 1rem;
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

			.optional {
				font-size: 0.8rem;
				color: #71717a;
				font-weight: 400;
			}
			.required {
				font-size: 0.8rem;
				color: var(--primary, #ff4655);
				font-weight: 500;
			}

			.request-link {
				font-size: 0.8rem;
				color: #71717a;
				font-weight: 400;

				a {
					color: var(--primary, #ff4655);
					text-decoration: none;
					transition: color 0.2s;

					&:hover {
						text-decoration: underline;
						color: #ff5f6d;
					}
				}
			}
		}

		.input-wrapper {
			position: relative;
			display: flex;
			align-items: center;
			width: 100%;

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
