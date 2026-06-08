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
		<section class="section">
			<div class="container wide">
		<div class="header">
			<span class="eyebrow">pricing</span>
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

					<button class="cta-btn btn btn-block {plan.highlight ? 'primary btn-primary' : 'secondary btn-ghost'}">
						{plan.cta}
					</button>
				</div>
			{/each}
		</div>
			</div>
		</section>
	</main>

	<Footer />

	<div class="bg-effects">
		<div class="glow-spot top"></div>
	</div>
</div>

<style lang="scss">
	.page-container {
		min-height: 100vh;
		position: relative;
		overflow: hidden;
		display: flex;
		flex-direction: column;
	}

	.content {
		position: relative;
		z-index: 10;
		flex: 1;

		.header {
			text-align: center;
			margin-bottom: var(--space-8);
			display: flex;
			flex-direction: column;
			align-items: center;
			gap: var(--space-4);

			h1 {
				font-size: var(--fs-h1);
				font-weight: var(--fw-bold);
			}

			p {
				color: var(--text-secondary);
				font-size: var(--fs-lg);
			}

			.currency-toggle {
				display: inline-flex;
				background: var(--tint-soft);
				padding: var(--space-1);
				border-radius: var(--radius-md);
				border: 1px solid var(--border-default);

				.toggle-btn {
					background: transparent;
					border: none;
					color: var(--text-secondary);
					padding: 0.5rem 1.5rem;
					border-radius: var(--radius-sm);
					cursor: pointer;
					font-weight: var(--fw-semibold);
					font-family: inherit;
					font-size: var(--fs-body);
					transition: background var(--dur) var(--ease), color var(--dur) var(--ease);

					&:hover {
						color: var(--text-primary);
					}

					&.active {
						background: var(--accent-gradient);
						color: #fff;
					}
				}
			}

			.storage-info {
				display: flex;
				flex-wrap: wrap;
				gap: var(--space-5);
				justify-content: center;
				font-size: var(--fs-sm);

				p {
					margin: 0;
					font-size: var(--fs-sm);
					color: var(--text-secondary);
				}

				.highlight {
					color: var(--primary);
					font-weight: var(--fw-semibold);
					margin-right: var(--space-1);
				}
			}
		}
	}

	.pricing-grid {
		display: flex;
		flex-wrap: wrap;
		justify-content: center;
		gap: var(--space-5);

		@media (max-width: 768px) {
			flex-direction: column;
			align-items: stretch;
		}

		.plan-card {
			flex: 1 1 300px;
			max-width: 450px;
			min-width: 280px;
			background: var(--bg-card);
			border: 1px solid var(--border-default);
			border-radius: var(--radius-md);
			box-shadow: var(--shadow-card);
			padding: var(--space-6);
			display: flex;
			flex-direction: column;
			gap: var(--space-6);
			transition: transform var(--dur) var(--ease), border-color var(--dur) var(--ease);

			@media (max-width: 768px) {
				max-width: none;
			}

			&:hover {
				transform: translateY(-2px);
				border-color: var(--border-strong);
			}

			&.highlight {
				background: linear-gradient(180deg, rgba(255, 70, 85, 0.1) 0%, var(--bg-card) 100%);
				border-color: rgba(255, 70, 85, 0.3);
				box-shadow: var(--shadow-glow);
			}

			.plan-header {
				h3 {
					margin: 0 0 var(--space-2);
					font-size: var(--fs-h3);
				}
				.price {
					display: flex;
					align-items: baseline;
					gap: var(--space-1);
					margin-bottom: var(--space-4);

					.amount {
						font-size: var(--fs-h2);
						font-weight: var(--fw-bold);
					}
					.period {
						color: var(--text-secondary);
					}
				}
				.description {
					margin: 0;
					color: var(--text-secondary);
					line-height: var(--lh-snug);
				}
			}

			.features {
				list-style: none;
				padding: 0;
				margin: 0;
				display: flex;
				flex-direction: column;
				gap: var(--space-4);
				flex: 1;

				li {
					display: flex;
					gap: var(--space-3);
					color: var(--text-secondary);

					:global(.check-icon) {
						color: var(--primary);
						flex-shrink: 0;
						margin-top: 4px;
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
