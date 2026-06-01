<script>
	import { enhance } from '$app/forms';
	import Icon from '@iconify/svelte';
	import SiloCatLogo from '$lib/assets/silo-cat.png';
	import Navbar from '$lib/components/Navbar.svelte';
	import Footer from '$lib/components/Footer.svelte';

	import { Turnstile } from 'svelte-turnstile';

	export let form;
	export let data;

	let loading = false;
	let turnstileRef;

	function handleSubmit() {
		loading = true;
		return async ({ update }) => {
			await update();
			loading = false;
			turnstileRef?.reset?.();
		};
	}
</script>

<svelte:head>
	<title>Early Access - SiloCat Sanctum</title>
</svelte:head>

<div class="landing-page">
	<Navbar />

	<!-- HERO SECTION -->
	<section class="hero-section">
		<div class="hero-content">
			<div class="logo-large">
				<img src={SiloCatLogo} alt="SiloCat Logo" />
			</div>
			<h1>The Sanctum awaits.</h1>
			<p class="tagline">
				Request an invite to get early access to a secure, no-nonsense file sharing platform
				designed for professionals. No trackers. No data mining. No compromises.
			</p>

			<div class="form-card">
				{#if form?.success}
					<div class="success-alert">
						<div class="icon-circle">
							<Icon icon="ri:checkbox-circle-line" width="32" />
						</div>
						<h2>Request Received</h2>
						<p>You're on the list! We'll email your invite code soon.</p>
					</div>
				{:else}
					<form method="POST" use:enhance={handleSubmit}>
						<div class="input-wrapper">
							<div class="input-icon">
								<Icon icon="ri:mail-line" width="20" />
							</div>
							<input
								type="email"
								name="email"
								placeholder="Enter your email to request early access invite"
								value={form?.email ?? ''}
								required
								class:error={form?.missing || form?.invalid}
							/>
							<button type="submit" disabled={loading}>
								{#if loading}
									<Icon icon="line-md:loading-loop" />
								{:else}
									<Icon icon="ri:arrow-right-line" />
								{/if}
							</button>
						</div>
						<div class="turnstile-wrapper">
							<Turnstile siteKey={data.turnstileSiteKey} bind:this={turnstileRef} theme="dark" />
						</div>
						{#if form?.error || form?.invalid}
							<div class="error-msg">
								{form?.error || 'Please enter a valid email address'}
							</div>
						{/if}
					</form>
				{/if}
				<p class="subtext">Limited spots available. First come, first served.</p>
			</div>
		</div>
	</section>

	<!-- BENEFITS SECTION -->
	<section class="benefits-section">
		<div class="benefits-grid">
			<div class="benefit-card">
				<div class="icon-box">
					<Icon icon="ri:shield-keyhole-line" width="32" />
				</div>
				<h3>Zero-Knowledge</h3>
				<p>
					Your files are encrypted before they leave your device. We can't see them, and neither can
					anyone else.
				</p>
			</div>
			<div class="benefit-card">
				<div class="icon-box">
					<Icon icon="ri:speed-mini-fill" width="32" />
				</div>
				<h3>Blazing Fast</h3>
				<p>
					Optimized parallel downloads and uploads ensure you get your content instantly, no matter
					the size.
				</p>
			</div>
			<div class="benefit-card">
				<div class="icon-box">
					<Icon icon="ri:spy-fill" width="32" />
				</div>
				<h3>Anonymous Identity</h3>
				<p>
					No phone numbers, no tracking. Your digital identity remains completely private within the
					Sanctum.
				</p>
			</div>
		</div>
	</section>

	<!-- BOTTOM CTA -->
	<section class="cta-section">
		<h2>Ready to secure your data?</h2>
		<p>Don't miss out on the future of encrypted storage.</p>
		<div class="cta-actions">
			<a href="/auth/signup" class="btn secondary">Create Account</a>
		</div>
	</section>

	<Footer />
</div>

<style lang="scss">
	:global(body) {
		background-color: #09090b;
		margin: 0;
		font-family: 'Outfit', sans-serif;
	}

	.landing-page {
		min-height: 100vh;
		display: flex;
		flex-direction: column;
		background: #09090b;
		color: white;
		overflow-x: hidden;
	}

	/* Navbar */
	/* Replaced by component */

	/* Hero Section */
	.hero-section {
		min-height: 80vh;
		padding: 120px 2rem 4rem; /* Space for fixed navbar + margins */
		display: flex;
		align-items: center;
		justify-content: center;
		position: relative;
		text-align: center;
		background: radial-gradient(circle at 50% 50%, rgba(255, 70, 85, 0.15) 0%, transparent 50%);

		&::before {
			content: '';
			position: absolute;
			top: 0;
			left: 0;
			right: 0;
			bottom: 0;
			background: url('https://grainy-gradients.vercel.app/noise.svg');
			opacity: 0.05;
			pointer-events: none;
		}

		/* ... rest of hero section styles ... */

		.hero-content {
			max-width: 600px;
			width: 100%;
			z-index: 1;
			display: flex;
			flex-direction: column;
			align-items: center;
			gap: 1.5rem;
		}

		.logo-large {
			width: 96px;
			height: 96px;
			background: radial-gradient(circle at center, rgba(255, 255, 255, 0.05), transparent);
			border: 1px solid rgba(255, 255, 255, 0.1);
			border-radius: 28px;
			display: flex;
			align-items: center;
			justify-content: center;
			box-shadow: 0 0 50px rgba(255, 70, 85, 0.2);
			margin-bottom: 1rem;

			img {
				width: 48px;
				height: 48px;
			}
		}

		h1 {
			font-size: 3.5rem;
			font-weight: 800;
			margin: 0;
			line-height: 1.1;
			background: linear-gradient(to bottom, #fff, #a1a1aa);
			-webkit-background-clip: text;
			-webkit-text-fill-color: transparent;
			background-clip: text; /* Standard property */
		}

		.tagline {
			font-size: 1.25rem;
			color: #a1a1aa;
			line-height: 1.6;
			margin: 0;
		}

		.form-card {
			background: rgba(255, 255, 255, 0.03);
			border: 1px solid rgba(255, 255, 255, 0.1);
			border-radius: 20px;
			width: 100%;
			max-width: 600px;
			margin-top: 2rem;
			backdrop-filter: blur(10px);
			padding: 1rem;
		}

		.subtext {
			font-size: 0.85rem;
			color: rgba(255, 255, 255, 0.4);
			margin-top: 1rem;
		}
	}

	/* Form Styling */
	.input-wrapper {
		position: relative;
		display: flex;
		align-items: center;
		background: rgba(0, 0, 0, 0.3);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: 12px;
		padding: 0.25rem;
		transition: border-color 0.2s;

		&:focus-within {
			border-color: #ff4655;
			box-shadow: 0 0 0 4px rgba(255, 70, 85, 0.1);
		}

		.input-icon {
			position: absolute;
			left: 1.5rem;
			color: #ff4655;
			pointer-events: none;
			display: flex;
			align-items: center;
			justify-content: center;
		}

		input {
			flex: 1;
			background: transparent;
			border: none;
			padding: 1rem 1rem 1rem 3rem;
			color: white;
			font-size: 1rem;
			outline: none;
			font-family: inherit;

			&::placeholder {
				color: rgba(255, 255, 255, 0.3);
			}
		}

		button {
			background: #ff4655;
			border: none;
			width: 42px;
			height: 42px;
			border-radius: 8px;
			color: white;
			cursor: pointer;
			display: flex;
			align-items: center;
			justify-content: center;
			margin-right: 0.25rem;
			transition: all 0.2s;

			&:hover:not(:disabled) {
				background: #e03e4b;
			}

			&:disabled {
				opacity: 0.7;
				cursor: not-allowed;
			}
		}
	}

	.error-msg {
		color: #ef4444;
		font-size: 0.9rem;
		margin-top: 0.5rem;
		text-align: left;
		padding-left: 0.5rem;
	}

	.success-alert {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.5rem;

		.icon-circle {
			color: #10b981;
			margin-bottom: 0.5rem;
		}

		h2 {
			margin: 0;
			font-size: 1.25rem;
		}

		p {
			margin: 0;
			color: #a1a1aa;
		}
	}

	/* Benefits Section */
	.benefits-section {
		padding: 6rem 2rem;
		background: rgba(0, 0, 0, 0.2);
		border-top: 1px solid rgba(255, 255, 255, 0.05);
		border-bottom: 1px solid rgba(255, 255, 255, 0.05);
	}

	.benefits-grid {
		max-width: 1200px;
		margin: 0 auto;
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
		gap: 3rem;
	}

	.benefit-card {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: 1rem;

		.icon-box {
			width: 56px;
			height: 56px;
			background: rgba(255, 70, 85, 0.1);
			color: #ff4655;
			border-radius: 16px;
			display: flex;
			align-items: center;
			justify-content: center;
			margin-bottom: 0.5rem;
		}

		h3 {
			font-size: 1.5rem;
			font-weight: 600;
			margin: 0;
		}

		p {
			color: #a1a1aa;
			line-height: 1.6;
			margin: 0;
		}
	}

	/* CTA Section */
	.cta-section {
		padding: 6rem 2rem;
		text-align: center;
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 1.5rem;

		h2 {
			font-size: 2.5rem;
			font-weight: 700;
			margin: 0;
		}

		p {
			color: #a1a1aa;
			font-size: 1.25rem;
			margin: 0;
		}

		.btn.secondary {
			margin-top: 1rem;
			padding: 1rem 2rem;
			background: rgba(255, 255, 255, 0.1);
			color: white;
			text-decoration: none;
			border-radius: 12px;
			font-weight: 600;
			border: 1px solid rgba(255, 255, 255, 0.1);
			transition: all 0.2s;

			&:hover {
				background: rgba(255, 255, 255, 0.15);
				border-color: rgba(255, 255, 255, 0.2);
			}
		}
	}

	/* Footer handled by component */

	@media (max-width: 768px) {
		.hero-section h1 {
			font-size: 2.5rem;
		}
		.benefits-grid {
			grid-template-columns: 1fr;
		}
	}
</style>
