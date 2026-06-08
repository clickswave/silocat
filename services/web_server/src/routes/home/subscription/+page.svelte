<script>
	import Icon from '@iconify/svelte';
	import { FrontendClient } from '$lib/frontendClient.js';
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';
	import { toast } from 'svelte-sonner';

	let { data } = $props();
	let isPro = $derived(data.user?.subscription?.name === 'Pro');
	let billingPeriod = 'monthly';
	let selectedCurrency = $state('USD'); // Default
	let selectedGateway = $state('razorpay');
	let processing = $state(false);
	let promoCode = $state('');
	let discount = $state(0);
	let discountApplied = $state(false);
	let verifiedCode = $state('');
	let validatingPromo = $state(false);

	const currencies = [
		{ code: 'USD', symbol: '$' },
		{ code: 'INR', symbol: '₹' }
	];

	// Pricing configuration
	const pricing = {
		USD: { Free: 0, Pro: 9 },
		INR: { Free: 0, Pro: 950 },
		EUR: { Free: 0, Pro: 8 }
	};

	const addonsData = {
		USD: [
			{ id: '1tb', name: '+1 TB Storage', price: 5, size: '1 TB' },
			{ id: '5tb', name: '+5 TB Storage', price: 20, size: '5 TB' },
			{ id: '10tb', name: '+10 TB Storage', price: 35, size: '10 TB' },
			{ id: '20tb', name: '+20 TB Storage', price: 60, size: '20 TB' }
		],
		INR: [
			{ id: '1tb', name: '+1 TB Storage', price: 399, size: '1 TB' },
			{ id: '5tb', name: '+5 TB Storage', price: 1599, size: '5 TB' },
			{ id: '10tb', name: '+10 TB Storage', price: 2999, size: '10 TB' },
			{ id: '20tb', name: '+20 TB Storage', price: 4999, size: '20 TB' }
		]
	};

	let addons = $derived(addonsData[selectedCurrency] || addonsData['USD']);

	const eurozone = [
		'austria',
		'at',
		'belgium',
		'be',
		'bulgaria',
		'bg',
		'croatia',
		'hr',
		'cyprus',
		'cy',
		'estonia',
		'ee',
		'finland',
		'fi',
		'france',
		'fr',
		'germany',
		'de',
		'greece',
		'gr',
		'ireland',
		'ie',
		'italy',
		'it',
		'latvia',
		'lv',
		'lithuania',
		'lt',
		'luxembourg',
		'lu',
		'malta',
		'mt',
		'netherlands',
		'nl',
		'portugal',
		'pt',
		'slovakia',
		'sk',
		'slovenia',
		'si',
		'spain',
		'es'
	];

	onMount(() => {
		// Detect country and set currency
		const country = data.user?.country?.toLowerCase();
		if (country === 'india' || country === 'in') {
			selectedCurrency = 'INR';
		} else if (eurozone.includes(country)) {
			selectedCurrency = 'EUR';
		} else {
			selectedCurrency = 'USD';
		}
	});

	const gateways = [
		{ id: 'razorpay', name: 'Razorpay', icon: 'simple-icons:razorpay', enabled: true },
		{ id: 'stripe', name: 'Stripe', icon: 'simple-icons:stripe', enabled: false },
		{ id: 'paypal', name: 'PayPal', icon: 'simple-icons:paypal', enabled: false },
		{ id: 'skrill', name: 'Skrill', icon: 'simple-icons:skrill', enabled: false }
	];

	async function checkPromo() {
		if (!promoCode || validatingPromo) return;
		validatingPromo = true;

		try {
			const res = await FrontendClient.post('/api/v1/billing/check-promo', {
				code: promoCode
			});

			if (res.data.status === 200 && res.data.data.valid) {
				discount = res.data.data.discount_percentage;
				discountApplied = true;
				verifiedCode = promoCode;
				toast.success(`Promo code applied! ${discount}% off.`);
			} else {
				discount = 0;
				discountApplied = false;
				verifiedCode = '';
				toast.error('Invalid promo code');
			}
		} catch (e) {
			console.error(e);
			const errorMessage = e.response?.data?.message || 'Failed to verify promo code';
			toast.error(errorMessage);
			discount = 0;
			discountApplied = false;
			verifiedCode = '';
		} finally {
			validatingPromo = false;
		}
	}

	async function handleSubscribe(type, identifier) {
		if (processing) return;
		processing = true;

		try {
			// 1. Create Order
			const orderRes = await FrontendClient.post('/api/v1/billing/order', {
				order_type: type,
				identifier: identifier,
				currency: selectedCurrency,
				gateway: selectedGateway,
				promo_code: discountApplied ? verifiedCode : null
			});

			// Handle 100% discount or free orders immediately
			// Handle 100% discount or free orders immediately
			if (orderRes.data.success && orderRes.data.success.amount === 0) {
				toast.success('Promo code applied! Upgrade successful.');
				setTimeout(() => location.reload(), 1500);
				return;
			}

			const { order_id, amount, currency, key_id } = orderRes.data.success;

			if (selectedGateway === 'razorpay') {
				// 2. Open Razorpay
				const options = {
					key: key_id,
					amount: amount,
					currency: currency,
					name: 'SiloCat Encrypted Storage',
					description: type === 'plan' ? `Upgrade to ${identifier}` : `Add-on: ${identifier}`,
					order_id: order_id,
					handler: async function (response) {
						// 3. Verify Payment
						try {
							await FrontendClient.post('/api/v1/billing/verify', {
								order_id: response.razorpay_order_id,
								payment_id: response.razorpay_payment_id,
								signature: response.razorpay_signature
							});
							toast.success('Payment successful! Benefits applied.');
							setTimeout(() => location.reload(), 1500);
						} catch (e) {
							console.error(e);
							toast.error('Payment verification failed. Contact support.');
							processing = false;
						}
					},
					prefill: {
						email: data.user?.email
					},
					theme: {
						color: '#ff4655'
					},
					modal: {
						ondismiss: function () {
							processing = false;
						}
					}
				};

				const rzp = new window.Razorpay(options);
				rzp.open();
			} else {
				toast.info('This gateway is coming soon!');
				processing = false;
			}
		} catch (e) {
			console.error(e);
			const errorMessage = e.response?.data?.message || 'Failed to initiate payment.';
			toast.error(errorMessage);
			processing = false;
		}
	}

	function formatPrice(amount) {
		const symbol = currencies.find((c) => c.code === selectedCurrency)?.symbol || '$';
		return `${symbol}${amount}`;
	}

	function calculateDiscountedPrice(price) {
		if (!discountApplied) return formatPrice(price);
		const discounted = Math.round(price * (1 - discount / 100));
		return formatPrice(discounted);
	}
</script>

<div class="subscription-page">
	<header class="page-header">
		<div class="header-top">
			<h1>Manage Subscription</h1>
		</div>
		<p class="subtitle">Upgrade your storage and security.</p>
	</header>

	<div class="promo-section">
		<h2>Have a Promo Code?</h2>
		<div class="input-group">
			<input
				type="text"
				placeholder="Enter code (e.g., 10-off-pro-1m)"
				bind:value={promoCode}
				disabled={discountApplied}
			/>
			{#if discountApplied}
				<button
					class="btn-apply remove"
					onclick={() => {
						discountApplied = false;
						discount = 0;
						verifiedCode = '';
						promoCode = '';
					}}>Remove</button
				>
			{:else}
				<button class="btn-apply" onclick={checkPromo} disabled={!promoCode || validatingPromo}>
					{validatingPromo ? '...' : 'Apply'}
				</button>
			{/if}
		</div>
		{#if discountApplied}
			<p class="promo-success">Code applied: {discount}% Discount</p>
		{/if}
	</div>

	<div class="payment-method-section">
		<h2>Payment Method</h2>
		<div class="gateways-grid">
			{#each gateways as gateway}
				<button
					class="gateway-card {selectedGateway === gateway.id ? 'selected' : ''} {gateway.enabled
						? ''
						: 'disabled'}"
					onclick={() => gateway.enabled && (selectedGateway = gateway.id)}
					disabled={!gateway.enabled}
				>
					<Icon icon={gateway.icon} width="32" class="gateway-icon" />
					<span class="gateway-name">{gateway.name}</span>
					{#if !gateway.enabled}
						<span class="coming-soon">Soon</span>
					{/if}
				</button>
			{/each}
		</div>
	</div>

	<div class="plans-section">
		<h2>Membership Plans</h2>
		<div class="pricing-grid">
			<!-- Free Plan -->
			<div class="card pricing-card">
				<div class="card-header">
					<h2>Free</h2>
					<p class="description">For personal use</p>
				</div>
				<div class="price">
					<span class="amount">{formatPrice(pricing[selectedCurrency].Free)}</span>
					<span class="period">/mo</span>
				</div>
				<ul class="features">
					<li><Icon icon="ri:checkbox-circle-fill" class="check-icon" /> 50 GB Storage</li>
					<li><Icon icon="ri:checkbox-circle-fill" class="check-icon" /> End-to-End Encryption</li>
				</ul>
				<button class="btn btn-outline" disabled={!isPro}>
					{isPro ? 'Downgrade' : 'Current Plan'}
				</button>
			</div>

			<!-- Pro Plan -->
			<div class="card pricing-card popular">
				<div class="popular-tag">Recommended</div>
				<div class="card-header">
					<h2>Pro</h2>
					<p class="description">For power users</p>
				</div>
				<div class="price">
					{#if discountApplied && pricing[selectedCurrency].Pro > 0}
						<span class="original-price">{formatPrice(pricing[selectedCurrency].Pro)}</span>
						<span class="amount">{calculateDiscountedPrice(pricing[selectedCurrency].Pro)}</span>
					{:else}
						<span class="amount">{formatPrice(pricing[selectedCurrency].Pro)}</span>
					{/if}
					<span class="period">/mo</span>
				</div>
				<ul class="features">
					<li><Icon icon="ri:checkbox-circle-fill" class="check-icon" /> 1 TB Storage</li>
					<li><Icon icon="ri:checkbox-circle-fill" class="check-icon" /> Priority Support</li>
					<li><Icon icon="ri:checkbox-circle-fill" class="check-icon" /> Zero Ads</li>
				</ul>
				<button
					class="btn btn-primary"
					disabled={isPro || processing}
					onclick={() => handleSubscribe('plan', 'pro')}
				>
					{isPro ? 'Current Plan' : processing ? 'Processing...' : 'Upgrade to Pro'}
				</button>
			</div>
		</div>
	</div>

	<div class="addons-section">
		<h2>Storage Add-ons</h2>
		<p class="section-desc">Need more space? Exclusive for Pro members.</p>

		<div class="addons-grid">
			{#each addons as addon}
				<div class="card addon-card {isPro ? '' : 'locked'}">
					{#if !isPro}
						<div class="lock-overlay">
							<Icon icon="ri:lock-fill" width="32" />
							<span>Pro Only</span>
						</div>
					{/if}
					<div class="addon-header">
						<h3>{addon.name}</h3>
						<span class="addon-size">{addon.size}</span>
					</div>
					<div class="addon-price">
						{formatPrice(addon.price)} <span class="period">/mo</span>
					</div>
					<button
						class="btn btn-secondary"
						disabled={!isPro || processing}
						onclick={() => handleSubscribe('quota', addon.id)}
					>
						{processing ? '...' : 'Add to Plan'}
					</button>
				</div>
			{/each}
		</div>
	</div>
</div>

<style lang="scss">
	.subscription-page {
		width: 100%;
		color: var(--text-primary);
	}

	.page-header {
		margin-bottom: var(--space-6);
		h1 {
			font-size: var(--fs-h3);
			font-weight: var(--fw-semibold);
			margin: 0 0 var(--space-1) 0;
		}
		.subtitle {
			color: var(--text-muted);
			font-size: var(--fs-sm);
		}
	}

	h2 {
		font-size: var(--fs-h3);
		margin-bottom: var(--space-5);
		font-weight: var(--fw-semibold);
	}
	.section-desc {
		color: var(--text-secondary);
		margin-bottom: var(--space-6);
		margin-top: calc(-1 * var(--space-4));
	}

	.pricing-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
		gap: var(--space-6);
		margin-bottom: var(--space-10);
	}

	.payment-method-section {
		margin-bottom: var(--space-8);
		text-align: left;

		h2 {
			font-size: var(--fs-lg);
			color: var(--text-secondary);
			margin-bottom: var(--space-4);
		}
	}

	.gateways-grid {
		display: flex;
		gap: var(--space-4);
		flex-wrap: wrap;
	}

	.gateway-card {
		background: var(--bg-card);
		border: 1px solid var(--border-default);
		border-radius: var(--radius-md);
		padding: var(--space-4) var(--space-5);
		display: flex;
		align-items: center;
		gap: var(--space-3);
		cursor: pointer;
		transition: border-color var(--dur) var(--ease), background var(--dur) var(--ease),
			color var(--dur) var(--ease);
		color: var(--text-primary);
		min-width: 140px;

		&:hover:not(.disabled) {
			border-color: var(--border-strong);
			background: var(--bg-card-hover);
		}

		&.selected {
			border-color: var(--primary);
			background: var(--tint-soft);
			color: var(--primary);
			.gateway-icon {
				color: var(--primary);
			}
		}

		&.disabled {
			opacity: 0.5;
			cursor: not-allowed;
			border-style: dashed;
		}

		.gateway-icon {
			color: var(--text-secondary);
		}

		.gateway-name {
			font-weight: var(--fw-semibold);
		}

		.coming-soon {
			font-size: var(--fs-xs);
			background: var(--tint-softer);
			padding: 2px 6px;
			border-radius: var(--radius-sm);
			color: var(--text-muted);
			margin-left: auto;
		}
	}

	.addons-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
		gap: var(--space-5);
	}

	.card {
		background: var(--bg-card);
		border: 1px solid var(--border-default);
		border-radius: var(--radius-md);
		box-shadow: var(--shadow-card);
		padding: var(--space-6);
		display: flex;
		flex-direction: column;
		position: relative;
		overflow: hidden;

		&.popular {
			border-color: var(--primary);
			background: linear-gradient(180deg, rgba(255, 70, 85, 0.06) 0%, var(--bg-card) 100%);
		}

		.popular-tag {
			background: var(--accent-gradient);
			color: #fff;
			font-size: var(--fs-xs);
			font-weight: var(--fw-semibold);
			padding: var(--space-1) var(--space-4);
			position: absolute;
			top: 12px;
			left: 50%;
			transform: translateX(-50%);
			border-radius: var(--radius-pill);
		}
	}

	.price {
		margin: var(--space-5) 0;
		.amount {
			font-size: var(--fs-h1);
			font-weight: var(--fw-black);
			font-family: var(--font-mono);
		}
		.period {
			color: var(--text-muted);
		}
	}

	.features {
		list-style: none;
		padding: 0;
		margin: 0 0 var(--space-6);
		li {
			display: flex;
			gap: var(--space-2);
			margin-bottom: var(--space-3);
			color: var(--text-secondary);
			.check-icon {
				color: var(--primary);
			}
		}
	}

	.addon-card {
		padding: var(--space-5);
		&.locked {
			opacity: 0.5;
			pointer-events: none;
		}

		.lock-overlay {
			position: absolute;
			inset: 0;
			background: rgba(0, 0, 0, 0.6);
			display: flex;
			flex-direction: column;
			align-items: center;
			justify-content: center;
			z-index: 10;
			font-weight: var(--fw-semibold);
			color: var(--text-primary);
		}

		.addon-header {
			display: flex;
			justify-content: space-between;
			align-items: center;
			margin-bottom: var(--space-4);
			h3 {
				margin: 0;
				font-size: var(--fs-lg);
			}
			.addon-size {
				background: var(--tint-softer);
				padding: 2px 8px;
				border-radius: var(--radius-sm);
				font-size: var(--fs-xs);
			}
		}

		.addon-price {
			font-size: var(--fs-h3);
			font-weight: var(--fw-bold);
			margin-bottom: var(--space-4);
			font-family: var(--font-mono);
			.period {
				font-size: var(--fs-sm);
				font-weight: var(--fw-regular);
				color: var(--text-muted);
			}
		}
	}

	.btn {
		width: 100%;
		padding: 0.75rem;
		border-radius: var(--radius-pill);
		font-weight: var(--fw-semibold);
		cursor: pointer;
		border: 1px solid transparent;
		margin-top: auto;
		transition: filter var(--dur) var(--ease), background var(--dur) var(--ease),
			border-color var(--dur) var(--ease);

		&.btn-primary {
			background: var(--accent-gradient);
			color: #fff;
			box-shadow: 0 6px 20px -6px var(--primary-glow);
			&:hover:not(:disabled) {
				filter: brightness(1.06);
			}
			&:disabled {
				opacity: 0.55;
				cursor: not-allowed;
			}
		}

		&.btn-secondary {
			background: var(--text-primary);
			color: var(--bg-app);
			&:hover:not(:disabled) {
				filter: brightness(0.92);
			}
			&:disabled {
				opacity: 0.55;
				cursor: not-allowed;
			}
		}

		&.btn-outline {
			background: var(--tint-soft);
			border-color: var(--border-default);
			color: var(--text-primary);
			&:hover:not(:disabled) {
				background: var(--tint-softer);
				border-color: var(--border-strong);
			}
			&:disabled {
				opacity: 0.55;
				cursor: not-allowed;
			}
		}
	}

	.promo-section {
		margin-bottom: var(--space-8);
		text-align: center;
		padding-bottom: var(--space-6);
		border-bottom: 1px solid var(--hairline);

		.input-group {
			display: flex;
			gap: var(--space-4);
			justify-content: center;
			align-items: center;
		}

		input {
			background: var(--bg-input);
			border: 1px solid var(--border-default);
			padding: 0.75rem 0.95rem;
			border-radius: var(--radius-sm);
			color: var(--text-primary);
			width: 100%;
			max-width: 300px;
			text-align: center;
			text-transform: uppercase;
			font-weight: var(--fw-semibold);
			letter-spacing: 1px;
			font-family: var(--font-mono);

			&:focus {
				outline: none;
				border-color: var(--primary);
				box-shadow: 0 0 0 3px var(--primary-glow);
			}

			&:disabled {
				opacity: 0.7;
				cursor: not-allowed;
			}
		}

		.btn-apply {
			padding: 0.75rem 1.5rem;
			border-radius: var(--radius-pill);
			border: 1px solid transparent;
			font-weight: var(--fw-semibold);
			cursor: pointer;
			background: var(--accent-gradient);
			color: #fff;
			box-shadow: 0 6px 20px -6px var(--primary-glow);
			transition: filter var(--dur) var(--ease), background var(--dur) var(--ease);

			&:hover:not(:disabled) {
				filter: brightness(1.06);
			}

			&:disabled {
				opacity: 0.55;
				cursor: not-allowed;
			}

			&.remove {
				background: var(--tint-soft);
				border-color: var(--border-default);
				color: var(--text-primary);
				box-shadow: none;
				&:hover {
					background: var(--tint-softer);
				}
			}
		}

		.promo-success {
			color: var(--success);
			margin-top: var(--space-4);
			font-weight: var(--fw-semibold);
		}
	}

	.original-price {
		text-decoration: line-through;
		color: var(--text-muted);
		font-size: var(--fs-h3);
		margin-right: var(--space-2);
		font-weight: var(--fw-semibold);
	}

	@media (max-width: 600px) {
		.promo-section .input-group {
			flex-direction: column;
		}
	}
</style>
