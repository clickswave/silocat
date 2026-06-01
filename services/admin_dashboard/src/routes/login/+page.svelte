<script>
	import { enhance } from '$app/forms';

	let { form } = $props();

	let email = $state(form?.email ?? '');
	let password = $state('');
	let loading = $state(false);

	// Update email if form returns it (e.g. on error)
	$effect(() => {
		if (form?.email) email = form.email;
	});
</script>

<div class="login-container">
	<div class="card">
		<div class="logo">
			<!-- Replace with your logo -->
			<div class="logo-placeholder"></div>
			<h1>Admin Portal</h1>
		</div>

		{#if form?.message}
			<div class="alert error">
				{form.message}
			</div>
		{/if}

		<form
			method="POST"
			use:enhance={() => {
				loading = true;
				return async ({ update }) => {
					await update();
					loading = false;
				};
			}}
		>
			<div class="input-group">
				<label for="email">Email</label>
				<input
					type="email"
					name="email"
					id="email"
					bind:value={email}
					placeholder="admin@silo.cat"
					required
				/>
			</div>

			<div class="input-group">
				<label for="password">Password</label>
				<input
					type="password"
					name="password"
					id="password"
					bind:value={password}
					placeholder="••••••••"
					required
				/>
			</div>

			<button type="submit" class="cta-button" disabled={loading}>
				{#if loading}
					Loading...
				{:else}
					Sign In
				{/if}
			</button>
		</form>
	</div>
</div>

<style lang="scss">
	.login-container {
		display: flex;
		justify-content: center;
		align-items: center;
		min-height: 100vh;
		background: var(--bg-app);
	}

	.card {
		background: var(--bg-card);
		padding: 3rem;
		border-radius: 24px;
		border: 1px solid var(--border-default);
		width: 100%;
		max-width: 400px;
		box-shadow: var(--shadow-card);
		text-align: center;

		.logo {
			margin-bottom: 2rem;
			display: flex;
			flex-direction: column;
			align-items: center;
			gap: 1rem;

			.logo-placeholder {
				width: 48px;
				height: 48px;
				background: var(--primary);
				border-radius: 12px;
				box-shadow: var(--shadow-glow);
			}

			h1 {
				font-size: 1.5rem;
				font-weight: 700;
				margin: 0;
			}
		}
	}

	form {
		display: flex;
		flex-direction: column;
		gap: 1.5rem;
	}

	.input-group {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		text-align: left;

		label {
			font-size: 0.9rem;
			color: var(--text-muted);
			font-weight: 500;
		}

		input {
			background: var(--bg-input);
			border: 1px solid var(--border-default);
			padding: 0.8rem 1rem;
			border-radius: 12px;
			color: var(--text-primary);
			font-family: inherit;
			outline: none;
			transition: all 0.2s;

			&:focus {
				border-color: var(--primary);
				box-shadow: 0 0 0 2px rgba(255, 70, 85, 0.1);
			}

			&::placeholder {
				color: rgba(255, 255, 255, 0.2);
			}
		}
	}

	.cta-button {
		background: var(--primary);
		color: white;
		border: none;
		padding: 1rem;
		border-radius: 12px;
		font-weight: 600;
		cursor: pointer;
		transition: all 0.2s;
		font-size: 1rem;
		margin-top: 0.5rem;

		&:hover:not(:disabled) {
			background: var(--primary-hover);
			transform: translateY(-1px);
			box-shadow: 0 4px 12px rgba(255, 70, 85, 0.3);
		}

		&:disabled {
			opacity: 0.7;
			cursor: not-allowed;
		}
	}

	.alert {
		padding: 1rem;
		border-radius: 8px;
		margin-bottom: 1.5rem;
		font-size: 0.9rem;
		font-weight: 500;

		&.error {
			background: rgba(255, 70, 85, 0.1);
			color: #ff4655;
			border: 1px solid rgba(255, 70, 85, 0.2);
		}
	}
</style>
