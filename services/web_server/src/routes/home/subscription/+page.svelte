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
		padding: 2rem;
		max-width: 1000px;
		margin: 0 auto;
		color: white;
	}

	.page-header {
		text-align: center;
		margin-bottom: 3rem;
		h1 {
			font-size: 2.5rem;
			font-weight: 700;
			margin: 0;
		}
		.subtitle {
			color: #a1a1aa;
			font-size: 1.1rem;
		}
	}

	h2 {
		font-size: 1.5rem;
		margin-bottom: 1.5rem;
		font-weight: 600;
	}
	.section-desc {
		color: #a1a1aa;
		margin-bottom: 2rem;
		margin-top: -1rem;
	}

	.pricing-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
		gap: 2rem;
		margin-bottom: 4rem;
	}

	.payment-method-section {
		margin-bottom: 3rem;
		text-align: left;

		h2 {
			font-size: 1.25rem;
			color: #a1a1aa;
			margin-bottom: 1rem;
		}
	}

	.gateways-grid {
		display: flex;
		gap: 1rem;
		flex-wrap: wrap;
	}

	.gateway-card {
		background: #18181b;
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: 12px;
		padding: 1rem 1.5rem;
		display: flex;
		align-items: center;
		gap: 0.75rem;
		cursor: pointer;
		transition: all 0.2s;
		color: white;
		min-width: 140px;

		&:hover:not(.disabled) {
			border-color: rgba(255, 255, 255, 0.3);
			background: rgba(255, 255, 255, 0.05);
		}

		&.selected {
			border-color: #ff4655;
			background: rgba(255, 70, 85, 0.1);
			color: #ff4655;
			.gateway-icon {
				color: #ff4655;
			}
		}

		&.disabled {
			opacity: 0.5;
			cursor: not-allowed;
			border-style: dashed;
		}

		.gateway-icon {
			color: #d4d4d8;
		}

		.gateway-name {
			font-weight: 600;
		}

		.coming-soon {
			font-size: 0.65rem;
			background: #27272a;
			padding: 2px 6px;
			border-radius: 4px;
			color: #a1a1aa;
			margin-left: auto;
		}
	}

	.addons-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
		gap: 1.5rem;
	}

	.card {
		background: #18181b;
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: 16px;
		padding: 2rem;
		display: flex;
		flex-direction: column;
		position: relative;
		overflow: hidden;

		&.popular {
			border-color: #ff4655;
			background: linear-gradient(180deg, rgba(255, 70, 85, 0.05) 0%, #18181b 100%);
		}

		.popular-tag {
			background: #ff4655;
			color: white;
			font-size: 0.75rem;
			font-weight: 600;
			padding: 0.25rem 1rem;
			position: absolute;
			top: 12px;
			left: 50%;
			transform: translateX(-50%);
			border-radius: 12px;
		}
	}

	.price {
		margin: 1.5rem 0;
		.amount {
			font-size: 2.5rem;
			font-weight: 800;
		}
		.period {
			color: #a1a1aa;
		}
	}

	.features {
		list-style: none;
		padding: 0;
		margin: 0 0 2rem;
		li {
			display: flex;
			gap: 0.5rem;
			margin-bottom: 0.75rem;
			color: #d4d4d8;
			.check-icon {
				color: #ff4655;
			}
		}
	}

	.addon-card {
		padding: 1.5rem;
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
			font-weight: 600;
		}

		.addon-header {
			display: flex;
			justify-content: space-between;
			align-items: center;
			margin-bottom: 1rem;
			h3 {
				margin: 0;
				font-size: 1.1rem;
			}
			.addon-size {
				background: rgba(255, 255, 255, 0.1);
				padding: 2px 8px;
				border-radius: 4px;
				font-size: 0.8rem;
			}
		}

		.addon-price {
			font-size: 1.25rem;
			font-weight: 700;
			margin-bottom: 1rem;
			.period {
				font-size: 0.9rem;
				font-weight: 400;
				color: #a1a1aa;
			}
		}
	}

	.btn {
		width: 100%;
		padding: 0.75rem;
		border-radius: 8px;
		font-weight: 600;
		cursor: pointer;
		border: none;
		margin-top: auto;
		transition: all 0.2s;

		&.btn-primary {
			background: #ff4655;
			color: white;
			&:hover {
				background: #e03e4b;
			}
			&:disabled {
				background: #3f3f46;
				cursor: not-allowed;
			}
		}

		&.btn-secondary {
			background: white;
			color: black;
			&:hover {
				background: #e4e4e7;
			}
		}

		&.btn-outline {
			background: transparent;
			border: 1px solid rgba(255, 255, 255, 0.2);
			color: white;
		}
	}

	.promo-section {
		margin-bottom: 3rem;
		text-align: center;
		padding-bottom: 2rem;
		border-bottom: 1px solid rgba(255, 255, 255, 0.1);

		.input-group {
			display: flex;
			gap: 1rem;
			justify-content: center;
			align-items: center;
		}

		input {
			background: #18181b;
			border: 1px solid rgba(255, 255, 255, 0.1);
			padding: 0.75rem;
			border-radius: 8px;
			color: white;
			width: 100%;
			max-width: 300px;
			text-align: center;
			text-transform: uppercase;
			font-weight: 600;
			letter-spacing: 1px;

			&:focus {
				outline: none;
				border-color: #ff4655;
			}

			&:disabled {
				opacity: 0.7;
				cursor: not-allowed;
			}
		}

		.btn-apply {
			padding: 0.75rem 1.5rem;
			border-radius: 8px;
			border: none;
			font-weight: 600;
			cursor: pointer;
			background: #ff4655;
			color: white;
			transition: all 0.2s;

			&:hover {
				background: #e03e4b;
			}

			&:disabled {
				background: #3f3f46;
				cursor: not-allowed;
			}

			&.remove {
				background: #3f3f46;
				&:hover {
					background: #27272a;
				}
			}
		}

		.promo-success {
			color: #4ade80;
			margin-top: 1rem;
			font-weight: 600;
		}
	}

	.original-price {
		text-decoration: line-through;
		color: #71717a;
		font-size: 1.5rem;
		margin-right: 0.5rem;
		font-weight: 600;
	}
</style>
