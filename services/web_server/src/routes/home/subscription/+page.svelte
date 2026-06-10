<script>
	import Icon from '@iconify/svelte';
	import { FrontendClient } from '$lib/frontendClient.js';
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';
	import { toast } from 'svelte-sonner';
	import { loadRazorpay } from '$lib/loadRazorpay.js';

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

				const Razorpay = await loadRazorpay();
				const rzp = new Razorpay(options);
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
		<div class="title-group">
			<h1>Subscription</h1>
			<p class="subtitle">Upgrade your storage and security.</p>
		</div>
		<div class="currency-switch" role="group" aria-label="Currency">
			{#each currencies as c}
				<button
					class={selectedCurrency === c.code ? 'active' : ''}
					onclick={() => (selectedCurrency = c.code)}
				>
					{c.symbol} {c.code}
				</button>
			{/each}
		</div>
	</header>

	<!-- Current plan status -->
	<div class="status-hero {isPro ? 'pro' : ''}">
		<div class="status-icon">
			<Icon icon={isPro ? 'ri:vip-crown-2-fill' : 'ri:shield-check-fill'} width="28" />
		</div>
		<div class="status-text">
			<span class="status-eyebrow">Current plan</span>
			<h2>{isPro ? 'Pro' : 'Free'}</h2>
			<p>
				{isPro
					? 'You have full access to Pro features and storage add-ons.'
					: 'You are on the free plan with 50 GB of end-to-end encrypted storage.'}
			</p>
		</div>
		{#if isPro}
			<div class="status-badge"><Icon icon="ri:vip-crown-2-fill" width="16" /> Pro member</div>
		{:else}
			<button class="hero-cta" disabled={processing} onclick={() => handleSubscribe('plan', 'pro')}>
				{processing ? 'Processing…' : 'Upgrade to Pro'}
			</button>
		{/if}
	</div>

	<section class="block">
		<div class="block-head"><h2>Plans</h2></div>
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
	</section>

	<section class="block">
		<div class="block-head">
			<h2>Storage add-ons</h2>
			<p class="muted">Need more space? Exclusive for Pro members.</p>
		</div>

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
	</section>

	<!-- Billing options: promo + payment -->
	<section class="block">
		<div class="block-head"><h2>Billing</h2></div>
		<div class="bill-grid">
			<div class="bill-card">
				<div class="bill-head">
					<Icon icon="ri:price-tag-3-line" width="18" />
					<h3>Promo code</h3>
				</div>
				<div class="promo-row">
					<input
						type="text"
						placeholder="Enter code (e.g. 10-off-pro-1m)"
						bind:value={promoCode}
						disabled={discountApplied}
					/>
					{#if discountApplied}
						<button
							class="promo-btn remove"
							onclick={() => {
								discountApplied = false;
								discount = 0;
								verifiedCode = '';
								promoCode = '';
							}}>Remove</button
						>
					{:else}
						<button class="promo-btn" onclick={checkPromo} disabled={!promoCode || validatingPromo}>
							{validatingPromo ? '…' : 'Apply'}
						</button>
					{/if}
				</div>
				{#if discountApplied}
					<p class="promo-success">
						<Icon icon="ri:checkbox-circle-fill" width="16" /> {discount}% discount applied
					</p>
				{/if}
			</div>

			<div class="bill-card">
				<div class="bill-head">
					<Icon icon="ri:bank-card-line" width="18" />
					<h3>Payment method</h3>
				</div>
				<div class="gateways-grid">
					{#each gateways as gateway}
						<button
							class="gateway-card {selectedGateway === gateway.id ? 'selected' : ''} {gateway.enabled
								? ''
								: 'disabled'}"
							onclick={() => gateway.enabled && (selectedGateway = gateway.id)}
							disabled={!gateway.enabled}
						>
							<Icon icon={gateway.icon} width="26" class="gateway-icon" />
							<span class="gateway-name">{gateway.name}</span>
							{#if !gateway.enabled}
								<span class="coming-soon">Soon</span>
							{/if}
						</button>
					{/each}
				</div>
			</div>
		</div>
	</section>
</div>

<style lang="scss">
	.subscription-page {
		width: 100%;
		color: var(--text-primary);
		display: flex;
		flex-direction: column;
		gap: var(--space-8);
	}

	.page-header {
		display: flex;
		justify-content: space-between;
		align-items: flex-end;
		gap: var(--space-4);
		flex-wrap: wrap;

		h1 {
			font-size: var(--fs-h3);
			font-weight: var(--fw-semibold);
			margin: 0 0 var(--space-1) 0;
		}
		.subtitle {
			color: var(--text-muted);
			font-size: var(--fs-sm);
			margin: 0;
		}
	}

	.currency-switch {
		display: flex;
		background: var(--tint-soft);
		border: 1px solid var(--border-default);
		border-radius: var(--radius-md);
		padding: 2px;
		gap: 2px;

		button {
			background: transparent;
			border: none;
			color: var(--text-muted);
			font-family: inherit;
			font-size: var(--fs-sm);
			font-weight: var(--fw-medium);
			padding: var(--space-2) var(--space-3);
			border-radius: var(--radius-sm);
			cursor: pointer;
			transition: background var(--dur) var(--ease), color var(--dur) var(--ease);
			&:hover {
				color: var(--text-primary);
			}
			&.active {
				background: var(--bg-elevated);
				color: var(--text-primary);
				box-shadow: var(--shadow-card);
			}
		}
	}

	/* Current plan hero */
	.status-hero {
		display: flex;
		align-items: center;
		gap: var(--space-4);
		background: var(--bg-card);
		border: 1px solid var(--border-default);
		border-radius: var(--radius-lg);
		padding: var(--space-5) var(--space-6);
		box-shadow: var(--shadow-card);

		&.pro {
			border-color: var(--primary);
			background: linear-gradient(110deg, rgba(255, 70, 85, 0.08), var(--bg-card) 60%);
		}

		.status-icon {
			display: grid;
			place-items: center;
			width: 54px;
			height: 54px;
			flex-shrink: 0;
			border-radius: var(--radius-md);
			background: var(--tint-soft);
			color: var(--text-secondary);
		}
		&.pro .status-icon {
			background: rgba(255, 70, 85, 0.12);
			color: var(--primary);
		}
		.status-text {
			flex: 1;
			min-width: 0;
			.status-eyebrow {
				font-size: var(--fs-xs);
				text-transform: uppercase;
				letter-spacing: 0.06em;
				color: var(--text-muted);
			}
			h2 {
				font-size: var(--fs-h3);
				font-weight: var(--fw-bold);
				margin: 2px 0 var(--space-1);
			}
			p {
				margin: 0;
				color: var(--text-secondary);
				font-size: var(--fs-sm);
			}
		}
		.status-badge {
			display: inline-flex;
			align-items: center;
			gap: var(--space-1);
			flex-shrink: 0;
			background: rgba(255, 70, 85, 0.12);
			color: var(--primary);
			font-weight: var(--fw-semibold);
			font-size: var(--fs-sm);
			padding: var(--space-2) var(--space-4);
			border-radius: var(--radius-pill, 999px);
		}
		.hero-cta {
			flex-shrink: 0;
			background: var(--accent-gradient);
			color: #fff;
			border: none;
			border-radius: var(--radius-pill, 999px);
			padding: var(--space-3) var(--space-5);
			font-family: inherit;
			font-weight: var(--fw-semibold);
			cursor: pointer;
			box-shadow: 0 6px 20px -6px var(--primary-glow);
			transition: filter var(--dur) var(--ease);
			&:hover:not(:disabled) {
				filter: brightness(1.06);
			}
			&:disabled {
				opacity: 0.6;
				cursor: not-allowed;
			}
		}
	}

	/* Section blocks */
	.block {
		display: flex;
		flex-direction: column;
		gap: var(--space-5);
	}
	.block-head {
		display: flex;
		align-items: baseline;
		gap: var(--space-3);
		flex-wrap: wrap;
		h2 {
			font-size: var(--fs-lg);
			font-weight: var(--fw-semibold);
			margin: 0;
		}
		.muted {
			color: var(--text-muted);
			font-size: var(--fs-sm);
			margin: 0;
		}
	}

	.pricing-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
		gap: var(--space-5);
	}

	/* Billing block */
	.bill-grid {
		display: grid;
		grid-template-columns: 1fr 1.2fr;
		gap: var(--space-5);
	}
	.bill-card {
		background: var(--bg-card);
		border: 1px solid var(--border-default);
		border-radius: var(--radius-md);
		padding: var(--space-5);
		box-shadow: var(--shadow-card);

		.bill-head {
			display: flex;
			align-items: center;
			gap: var(--space-2);
			margin-bottom: var(--space-4);
			color: var(--text-secondary);
			h3 {
				margin: 0;
				font-size: var(--fs-body);
				font-weight: var(--fw-semibold);
				color: var(--text-primary);
			}
		}
	}
	.promo-row {
		display: flex;
		gap: var(--space-2);
		input {
			flex: 1;
			min-width: 0;
			background: var(--bg-input);
			border: 1px solid var(--border-default);
			padding: 0.7rem 0.9rem;
			border-radius: var(--radius-sm);
			color: var(--text-primary);
			font-family: var(--font-mono);
			text-transform: uppercase;
			letter-spacing: 0.04em;
			outline: none;
			&:focus {
				border-color: var(--primary);
				box-shadow: 0 0 0 3px var(--primary-glow);
			}
			&:disabled {
				opacity: 0.7;
			}
		}
		.promo-btn {
			flex: none;
			padding: 0 var(--space-4);
			border-radius: var(--radius-sm);
			border: 1px solid transparent;
			font-family: inherit;
			font-weight: var(--fw-semibold);
			cursor: pointer;
			background: var(--accent-gradient);
			color: #fff;
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
			}
		}
	}
	.promo-success {
		display: flex;
		align-items: center;
		gap: var(--space-1);
		color: var(--success);
		font-size: var(--fs-sm);
		font-weight: var(--fw-medium);
		margin: var(--space-3) 0 0;
	}

	.gateways-grid {
		display: flex;
		gap: var(--space-3);
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

	.original-price {
		text-decoration: line-through;
		color: var(--text-muted);
		font-size: var(--fs-h3);
		margin-right: var(--space-2);
		font-weight: var(--fw-semibold);
	}

	@media (max-width: 760px) {
		.bill-grid {
			grid-template-columns: 1fr;
		}
	}

	@media (max-width: 600px) {
		.page-header {
			align-items: flex-start;
		}
		.status-hero {
			flex-wrap: wrap;
			.hero-cta,
			.status-badge {
				width: 100%;
				justify-content: center;
			}
		}
	}
</style>
