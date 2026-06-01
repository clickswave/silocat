<script>
	import Icon from '@iconify/svelte';
	import Navbar from '$lib/components/Navbar.svelte';
	import Footer from '$lib/components/Footer.svelte';

	let selectedCurrency = $state('USD');

	const pricingData = {
		USD: { price: '$9', period: '/month', symbol: '$' },
		EUR: { price: '€8', period: '/month', symbol: '€' },
		INR: { price: '₹950', period: '/month', symbol: '₹' }
	};

	let currentPricing = $derived(pricingData[selectedCurrency]);

	let plans = $derived([
		{
			name: 'Free',
			price: '0',
			currencySymbol: currentPricing.symbol,
			period: '/month',
			description: 'For the wandering dev who needs a quick drop.',
			features: [
				'20GB Shadow Limit (Anon)',
				'50GB Sanctum Limit (Auth)',
				'7-Day Retention',
				'ChaCha20 Encryption',
				'Watched by WatchCat'
			],
			cta: 'Get Started',
			highlight: false
		},
		{
			name: 'Pro',
			price: selectedCurrency === 'INR' ? '950' : selectedCurrency === 'EUR' ? '8' : '9',
			currencySymbol: currentPricing.symbol,
			period: '/month',
			description: 'For technical beasts who roam the digital wild.',
			features: [
				'Unlimited Shadow Uploads',
				'1TB Sanctum Vault',
				'Custom Expiry & Passwords',
				'Priority Network Paths',
				'Direct Access'
			],
			cta: 'Go Pro',
			highlight: true
		},
		{
			name: 'Enterprise',
			price: 'Custom',
			currencySymbol: '',
			period: '',
			description: 'Managed territories for your entire pack.',
			features: [
				'Unlimited Everything',
				'SSO / SAML Integration',
				'Audit Trails & Compliance',
				'Custom Retention Policies',
				'Dedicated Support',
				'DMCA / Legal Compliance Tools'
			],
			cta: 'Contact Sales',
			highlight: false
		}
	]);

	const currencies = [
		{ code: 'USD', label: 'USD' },
		{ code: 'EUR', label: 'EUR' },
		{ code: 'INR', label: 'INR' }
	];
</script>

<svelte:head>
	<title>SiloCat Pricing - Simple, Transparent, Secure</title>
	<meta
		name="description"
		content="Flexible plans for anonymous and power users. Kitty powered E2E encrypted anonymous file-sharing and cloud storage."
	/>
	<meta property="og:title" content="SiloCat Pricing - Simple, Transparent, Secure" />
	<meta
		property="og:description"
		content="Kitty powered E2E encrypted anonymous file-sharing and cloud storage platform with parallel downloads."
	/>
</svelte:head>

<div class="page-container">
	<Navbar />

	<main class="content">
		<div class="header">
			<h1>Simple, Transparent <span class="text-gradient">Pricing</span></h1>
			<p>Choose the plan that fits your needs.</p>

			<div class="currency-toggle">
				{#each currencies as currency}
					<button
						class="toggle-btn {selectedCurrency === currency.code ? 'active' : ''}"
						onclick={() => (selectedCurrency = currency.code)}
					>
						{currency.label}
					</button>
				{/each}
			</div>

			<div class="storage-info">
				<p>
					<span class="highlight">Volatile:</span> Temporary storage, auto-deleted after 7 days.
				</p>
				<p>
					<span class="highlight">Permanent:</span> Secure storage, kept indefinitely until you delete
					it.
				</p>
			</div>
		</div>

		<div class="pricing-grid">
			{#each plans as plan}
				<div class="plan-card {plan.highlight ? 'highlight' : ''}">
					<div class="plan-header">
						<h3>{plan.name}</h3>
						<div class="price">
							{#if plan.price === 'Custom'}
								<span class="amount">Custom</span>
							{:else}
								<span class="amount">{plan.currencySymbol}{plan.price}</span>
								<span class="period">{plan.period}</span>
							{/if}
						</div>
						<p class="description">{plan.description}</p>
					</div>

					<ul class="features">
						{#each plan.features as feature}
							<li>
								<Icon icon="ri:checkbox-circle-fill" class="check-icon" />
								{feature}
							</li>
						{/each}
					</ul>

					<button class="cta-btn {plan.highlight ? 'primary' : 'secondary'}">
						{plan.cta}
					</button>
				</div>
			{/each}
		</div>
	</main>

	<Footer />

	<div class="bg-effects">
		<div class="glow-spot top"></div>
	</div>
</div>

<style lang="scss">
	:global(body) {
		margin: 0;
		background-color: #0b0b0d;
		color: white;
		font-family: 'Outfit', sans-serif;
	}

	.page-container {
		min-height: 100vh;
		position: relative;
		overflow: hidden;
		display: flex;
		flex-direction: column;
	}

	.content {
		padding: 4rem 2rem;
		position: relative;
		z-index: 10;
		max-width: 1600px;
		margin: 0 auto;
		flex: 1;

		.header {
			text-align: center;
			margin-bottom: 4rem;

			h1 {
				font-size: 3rem;
				font-weight: 700;
				margin-bottom: 1rem;

				.text-gradient {
					background: linear-gradient(135deg, #fff 0%, #a1a1aa 100%);
					-webkit-background-clip: text;
					-webkit-text-fill-color: transparent;
				}
			}

			p {
				color: #a1a1aa;
				font-size: 1.2rem;
			}

			.currency-toggle {
				margin: 2rem auto;
				display: inline-flex;
				background: rgba(255, 255, 255, 0.05);
				padding: 4px;
				border-radius: 12px;
				border: 1px solid rgba(255, 255, 255, 0.1);

				.toggle-btn {
					background: transparent;
					border: none;
					color: #a1a1aa;
					padding: 0.5rem 1.5rem;
					border-radius: 8px;
					cursor: pointer;
					font-weight: 600;
					transition: all 0.2s;

					&:hover {
						color: white;
					}

					&.active {
						background: var(--primary, #ff4655);
						color: white;
					}
				}
			}

			.storage-info {
				margin-top: 1.5rem;
				display: flex;
				gap: 2rem;
				justify-content: center;
				font-size: 0.95rem;

				p {
					margin: 0;
					font-size: 0.9rem;
					color: #d4d4d8;
				}

				.highlight {
					color: var(--primary, #ff4655);
					font-weight: 600;
					margin-right: 0.25rem;
				}
			}
		}
	}

	.pricing-grid {
		display: flex;
		flex-wrap: wrap;
		justify-content: center;
		gap: 2rem;

		.plan-card {
			flex: 1 1 300px;
			max-width: 450px;
			min-width: 280px;
			background: rgba(255, 255, 255, 0.03);
			border: 1px solid rgba(255, 255, 255, 0.08);
			border-radius: 24px;
			padding: 2.5rem;
			display: flex;
			flex-direction: column;
			gap: 2rem;
			transition: transform 0.3s;

			&:hover {
				transform: translateY(-5px);
				background: rgba(255, 255, 255, 0.05);
			}

			&.highlight {
				background: linear-gradient(180deg, rgba(255, 70, 85, 0.1) 0%, rgba(20, 20, 22, 0.6) 100%);
				border-color: rgba(255, 70, 85, 0.3);
				box-shadow: 0 0 40px rgba(255, 70, 85, 0.15);
			}

			.plan-header {
				h3 {
					margin: 0;
					font-size: 1.5rem;
					margin-bottom: 0.5rem;
				}
				.price {
					display: flex;
					align-items: baseline;
					gap: 0.25rem;
					margin-bottom: 1rem;

					.amount {
						font-size: 2.5rem;
						font-weight: 700;
					}
					.period {
						color: #a1a1aa;
					}
				}
				.description {
					margin: 0;
					color: #a1a1aa;
					line-height: 1.5;
				}
			}

			.features {
				list-style: none;
				padding: 0;
				margin: 0;
				display: flex;
				flex-direction: column;
				gap: 1rem;
				flex: 1;

				li {
					display: flex;
					gap: 0.75rem;
					color: #d4d4d8;

					:global(.check-icon) {
						color: var(--primary, #ff4655);
						flex-shrink: 0;
						margin-top: 4px;
					}
				}
			}

			.cta-btn {
				width: 100%;
				padding: 1rem;
				border-radius: 12px;
				font-weight: 600;
				font-size: 1rem;
				cursor: pointer;
				border: none;
				transition: all 0.2s;

				&.primary {
					background: var(--primary, #ff4655);
					color: white;
					&:hover {
						background: #e03e4b;
					}
				}

				&.secondary {
					background: rgba(255, 255, 255, 0.1);
					color: white;
					&:hover {
						background: rgba(255, 255, 255, 0.2);
					}
				}
			}
		}
	}

	.bg-effects {
		position: absolute;
		inset: 0;
		z-index: 0;
		pointer-events: none;

		.glow-spot {
			position: absolute;
			width: 800px;
			height: 800px;
			background: radial-gradient(circle, rgba(255, 70, 85, 0.1) 0%, transparent 70%);
			filter: blur(100px);

			&.top {
				top: -40%;
				left: 50%;
				transform: translateX(-50%);
			}
		}
	}
</style>
