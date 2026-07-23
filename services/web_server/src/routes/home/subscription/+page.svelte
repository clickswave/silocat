<script>
	import Icon from '@iconify/svelte';
	import { FrontendClient } from '$lib/frontendClient.js';
	import { onMount } from 'svelte';
	import { browser } from '$app/environment';
	import { toast } from 'svelte-sonner';
	import { createQuery } from '@tanstack/svelte-query';
	import { loadRazorpay } from '$lib/loadRazorpay.js';
	import { Button, Segmented, Badge, Progress, Input } from '$lib/ui';

	let { data } = $props();

	let planName = $derived(data.user?.subscription?.name || 'Free');
	let isPro = $derived(planName === 'Pro');
	let isPlus = $derived(planName === 'Plus');
	let isPaid = $derived(isPro || isPlus);
	let expiresOn = $derived(
		data.user?.subscription?.expires_on
			? new Date(data.user.subscription.expires_on).toLocaleDateString(undefined, {
					month: 'short',
					day: 'numeric',
					year: 'numeric'
				})
			: null
	);

	let selectedCurrency = $state('USD');
	let cycle = $state('monthly'); // monthly | annual
	const selectedGateway = 'razorpay';
	let processing = $state(null); // identifier currently processing

	let promoCode = $state('');
	let discount = $state(0);
	let discountApplied = $state(false);
	let verifiedCode = $state('');
	let validatingPromo = $state(false);

	const SYMBOL = { USD: '$', INR: '₹', EUR: '€' };
	const symbol = $derived(SYMBOL[selectedCurrency] || '$');

	// Mirrors api_switch calculate_price(). Annual = ~10 months (two months free).
	const PRICES = {
		USD: { plus: { monthly: 4, annual: 39 }, pro: { monthly: 10, annual: 96 } },
		INR: { plus: { monthly: 349, annual: 3490 }, pro: { monthly: 899, annual: 8990 } },
		EUR: { plus: { monthly: 4, annual: 39 }, pro: { monthly: 9, annual: 90 } }
	};

	const eurozone = new Set([
		'austria','at','belgium','be','bulgaria','bg','croatia','hr','cyprus','cy','estonia','ee',
		'finland','fi','france','fr','germany','de','greece','gr','ireland','ie','italy','it','latvia',
		'lv','lithuania','lt','luxembourg','lu','malta','mt','netherlands','nl','portugal','pt','slovakia',
		'sk','slovenia','si','spain','es'
	]);

	onMount(() => {
		const country = data.user?.country?.toLowerCase();
		if (country === 'india' || country === 'in') selectedCurrency = 'INR';
		else if (eurozone.has(country)) selectedCurrency = 'EUR';
		else selectedCurrency = 'USD';
	});

	// Live storage for the current-plan hero (shared query key: stays in sync with
	// the sidebar + dashboard, updates after any upload/delete).
	const storageQuery = createQuery(() => ({
		queryKey: ['fetchStorageStats'],
		queryFn: async () => {
			const { data } = await FrontendClient.get('/api/v1/sanctum/user/storage');
			return data?.success || { used: 0, total: 0 };
		},
		enabled: browser
	}));
	let usage = $derived({
		used: storageQuery.data?.used || 0,
		total: storageQuery.data?.total || data.user?.totalAvailableSpace || 0
	});

	function fmtSize(bytes) {
		if (!bytes) return '0 B';
		const k = 1024;
		const s = ['B', 'KB', 'MB', 'GB', 'TB'];
		const i = Math.min(Math.floor(Math.log(bytes) / Math.log(k)), s.length - 1);
		return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + s[i];
	}
	let usedPct = $derived(usage.total ? Math.min((usage.used / usage.total) * 100, 100) : 0);

	function priceOf(planId) {
		if (planId === 'free') return 0;
		return PRICES[selectedCurrency]?.[planId]?.[cycle] ?? 0;
	}
	function displayPrice(planId) {
		const p = priceOf(planId);
		if (discountApplied && p > 0) return Math.round(p * (1 - discount / 100));
		return p;
	}
	const suffix = $derived(cycle === 'annual' ? '/yr' : '/mo');

	// Storage is the only thing you pay for. Every privacy feature (encryption,
	// password + expiring share links, anonymous drops) is on every tier, so the
	// plans differ by space, not by locking features behind a paywall.
	const plans = $derived([
		{
			id: 'free',
			name: 'Free',
			tagline: 'Everything, 10 GB of space.',
			features: ['10 GB encrypted storage', 'End-to-end encryption', 'Password + expiring share links', 'Up to 20 GB anonymous drops']
		},
		{
			id: 'plus',
			name: 'Plus',
			tagline: '20× the space.',
			features: ['200 GB encrypted storage', 'Everything in Free', 'Email support']
		},
		{
			id: 'pro',
			name: 'Pro',
			tagline: 'Room for everything.',
			recommended: true,
			features: ['2 TB encrypted storage', 'Everything in Free', 'Priority support']
		}
	]);

	// What the button on each plan card should do given the user's current plan.
	function planCta(planId) {
		if (planId === 'free') {
			return { label: isPaid ? 'Included' : 'Current plan', disabled: true, variant: 'ghost' };
		}
		if (planId === 'plus') {
			if (isPlus) return { label: 'Current plan', disabled: true, variant: 'ghost' };
			if (isPro) return { label: 'Included', disabled: true, variant: 'ghost' };
			return { label: 'Choose Plus', disabled: false, variant: 'ghost', action: () => subscribe('plan', 'plus') };
		}
		// pro
		if (isPro) return { label: 'Current plan', disabled: true, variant: 'ghost' };
		return { label: isPlus ? 'Upgrade to Pro' : 'Go Pro', disabled: false, variant: 'solid', action: () => subscribe('plan', 'pro') };
	}

	async function checkPromo() {
		if (!promoCode || validatingPromo) return;
		validatingPromo = true;
		try {
			const res = await FrontendClient.post('/api/v1/billing/check-promo', { code: promoCode });
			if (res.data.status === 200 && res.data.data.valid) {
				discount = res.data.data.discount_percentage;
				discountApplied = true;
				verifiedCode = promoCode;
				toast.success(`Promo applied: ${discount}% off.`);
			} else {
				toast.error('Invalid promo code');
			}
		} catch (e) {
			toast.error(e.response?.data?.message || 'Failed to verify promo code');
		} finally {
			validatingPromo = false;
		}
	}

	function clearPromo() {
		discountApplied = false;
		discount = 0;
		verifiedCode = '';
		promoCode = '';
	}

	async function subscribe(type, identifier) {
		if (processing) return;
		processing = identifier;
		try {
			const orderRes = await FrontendClient.post('/api/v1/billing/order', {
				order_type: type,
				identifier,
				currency: selectedCurrency,
				gateway: selectedGateway,
				cycle,
				promo_code: discountApplied ? verifiedCode : null
			});

			if (orderRes.data.success && orderRes.data.success.amount === 0) {
				toast.success('Applied. Enjoy your new plan.');
				setTimeout(() => location.reload(), 1200);
				return;
			}

			const { order_id, amount, currency, key_id } = orderRes.data.success;
			const Razorpay = await loadRazorpay();
			const rzp = new Razorpay({
				key: key_id,
				amount,
				currency,
				name: 'Silocat',
				description: type === 'plan' ? `${identifier} (${cycle})` : `Add-on: ${identifier}`,
				order_id,
				handler: async (response) => {
					try {
						await FrontendClient.post('/api/v1/billing/verify', {
							order_id: response.razorpay_order_id,
							payment_id: response.razorpay_payment_id,
							signature: response.razorpay_signature
						});
						toast.success('Payment successful. Benefits applied.');
						setTimeout(() => location.reload(), 1200);
					} catch (e) {
						toast.error('Payment verification failed. Contact support.');
						processing = null;
					}
				},
				prefill: { email: data.user?.email },
				theme: { color: '#ff4655' },
				modal: { ondismiss: () => (processing = null) }
			});
			rzp.open();
		} catch (e) {
			toast.error(e.response?.data?.message || 'Failed to start checkout.');
			processing = null;
		}
	}
</script>

<div class="view sub-page">
	<header class="page-head">
		<div>
			<h1 class="page-title">Plans</h1>
			<p class="page-subtitle">Every feature is free. Upgrade only for more space.</p>
		</div>
		<Segmented
			bind:value={selectedCurrency}
			size="sm"
			options={[
				{ value: 'USD', label: 'USD' },
				{ value: 'EUR', label: 'EUR' },
				{ value: 'INR', label: 'INR' }
			]}
		/>
	</header>

	<!-- Current plan + live usage -->
	<div class="current">
		<div class="current-main">
			<span class="cur-eyebrow">Current plan</span>
			<div class="cur-name">
				{planName}
				{#if isPaid}<Badge tone="accent">Active</Badge>{/if}
			</div>
			{#if isPaid && expiresOn}
				<span class="cur-expiry">Active until {expiresOn} · renew anytime, no auto-charge</span>
			{/if}
		</div>
		<div class="current-usage">
			<div class="usage-top">
				<span>{fmtSize(usage.used)} used</span>
				<span class="muted">of {fmtSize(usage.total)}</span>
			</div>
			<Progress value={usedPct} size="sm" tone={usedPct > 90 ? 'warn' : 'accent'} label="Storage used" />
		</div>
	</div>

	<!-- Billing cycle -->
	<div class="cycle-row">
		<Segmented
			bind:value={cycle}
			options={[
				{ value: 'monthly', label: 'Monthly' },
				{ value: 'annual', label: 'Annual' }
			]}
		/>
		<span class="save-hint">Save ~17% with annual billing (2 months free)</span>
	</div>

	<!-- Plans -->
	<div class="plan-grid">
		{#each plans as plan (plan.id)}
			{@const cta = planCta(plan.id)}
			{@const isCurrent = plan.name === planName}
			<div class="plan" class:featured={plan.recommended} class:current={isCurrent}>
				<div class="plan-top">
					<div class="plan-name">
						<h3>{plan.name}</h3>
						{#if plan.recommended}<Badge tone="accent">Recommended</Badge>{/if}
						{#if isCurrent}<Badge tone="ok">Current</Badge>{/if}
					</div>
					<p class="plan-tagline">{plan.tagline}</p>
					<div class="plan-price">
						{#if discountApplied && priceOf(plan.id) > 0}
							<span class="was">{symbol}{priceOf(plan.id)}</span>
						{/if}
						<span class="amount">{symbol}{displayPrice(plan.id)}</span>
						<span class="per">{plan.id === 'free' ? '/mo' : suffix}</span>
					</div>
					{#if plan.id !== 'free' && cycle === 'annual'}
						<span class="billed-note">billed annually</span>
					{/if}
				</div>

				<ul class="plan-features">
					{#each plan.features as f (f)}
						<li><Icon icon="ri:check-line" width="15" class="ck" /> {f}</li>
					{/each}
				</ul>

				<Button
					block
					variant={cta.variant}
					disabled={cta.disabled || processing === plan.id}
					loading={processing === plan.id}
					onclick={cta.action}
				>
					{cta.label}
				</Button>
			</div>
		{/each}
	</div>

	<!-- Checkout helpers: promo + trust -->
	<div class="checkout-row">
		<div class="promo">
			{#if discountApplied}
				<div class="promo-applied">
					<Icon icon="ri:checkbox-circle-fill" width="16" />
					<span><b>{verifiedCode}</b> · {discount}% off</span>
					<button class="promo-clear" onclick={clearPromo}>Remove</button>
				</div>
			{:else}
				<div class="promo-input">
					<Input bind:value={promoCode} placeholder="Promo code" icon="ri:price-tag-3-line" />
					<Button variant="ghost" loading={validatingPromo} disabled={!promoCode} onclick={checkPromo}>
						Apply
					</Button>
				</div>
			{/if}
		</div>
		<div class="trust">
			<Icon icon="ri:shield-check-line" width="15" />
			Payments secured by Razorpay
		</div>
	</div>

	<p class="prepaid-note">
		Plans are prepaid for the term you pick. No card is stored and nothing auto-renews, you're
		never charged without clicking. We'll remind you before your plan ends.
	</p>
</div>

<style lang="scss">
	.sub-page {
		gap: var(--space-5);
	}

	/* current plan strip */
	.current {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-6);
		flex-wrap: wrap;
		background: var(--surface);
		border: 1px solid var(--edge);
		border-radius: var(--radius-md);
		padding: var(--space-4) var(--space-5);
	}
	.cur-eyebrow {
		font-size: var(--fs-xs);
		color: var(--ink-faint);
	}
	.cur-name {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		font-size: var(--fs-h3);
		font-weight: var(--fw-semibold);
		margin-top: 2px;
	}
	.cur-expiry {
		display: block;
		margin-top: var(--space-1);
		font-size: var(--fs-xs);
		color: var(--ink-faint);
	}
	.current-usage {
		flex: 1;
		min-width: 220px;
		max-width: 360px;
	}
	.usage-top {
		display: flex;
		justify-content: space-between;
		font-family: var(--font-mono);
		font-size: var(--fs-xs);
		color: var(--ink-mute);
		margin-bottom: var(--space-2);
		.muted {
			color: var(--ink-faint);
		}
	}

	/* cycle toggle */
	.cycle-row {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		flex-wrap: wrap;
	}
	.save-hint {
		font-size: var(--fs-sm);
		color: var(--ink-faint);
	}

	/* plan grid */
	.plan-grid {
		display: grid;
		grid-template-columns: repeat(3, 1fr);
		gap: var(--space-4);
		align-items: stretch;

		@media (max-width: 860px) {
			grid-template-columns: 1fr;
			max-width: 460px;
		}
	}
	.plan {
		display: flex;
		flex-direction: column;
		gap: var(--space-5);
		background: var(--surface);
		border: 1px solid var(--edge);
		border-radius: var(--radius-md);
		padding: var(--space-6);

		&.featured {
			border-color: var(--accent);
		}
	}
	.plan-top {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}
	.plan-name {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		h3 {
			font-size: var(--fs-body);
			font-weight: var(--fw-semibold);
		}
	}
	.plan-tagline {
		font-size: var(--fs-sm);
		color: var(--ink-mute);
		margin: 0;
	}
	.plan-price {
		display: flex;
		align-items: baseline;
		gap: var(--space-1);
		margin-top: var(--space-2);
		.was {
			text-decoration: line-through;
			color: var(--ink-faint);
			font-size: var(--fs-lg);
			margin-right: var(--space-1);
		}
		.amount {
			font-size: var(--fs-h1);
			font-weight: var(--fw-semibold);
			letter-spacing: var(--tracking-tight);
		}
		.per {
			color: var(--ink-faint);
			font-size: var(--fs-sm);
		}
	}
	.billed-note {
		font-size: var(--fs-xs);
		color: var(--ink-faint);
	}
	.plan-features {
		list-style: none;
		padding: 0;
		margin: 0;
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		flex: 1;
		li {
			display: flex;
			gap: var(--space-2);
			font-size: var(--fs-sm);
			color: var(--ink-mute);
			line-height: var(--lh-snug);
			:global(.ck) {
				color: var(--ink-faint);
				flex-shrink: 0;
				margin-top: 2px;
			}
		}
	}

	/* checkout helpers */
	.checkout-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-4);
		flex-wrap: wrap;
	}
	.promo {
		flex: 1;
		min-width: 260px;
		max-width: 420px;
	}
	.promo-input {
		display: flex;
		gap: var(--space-2);
	}
	.promo-applied {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		font-size: var(--fs-sm);
		color: var(--ok);
		background: var(--ok-soft);
		border-radius: var(--radius-sm);
		padding: var(--space-2) var(--space-3);
		b {
			font-weight: var(--fw-semibold);
		}
	}
	.promo-clear {
		margin-left: auto;
		background: none;
		border: none;
		color: var(--ink-mute);
		font-family: inherit;
		font-size: var(--fs-sm);
		cursor: pointer;
		&:hover {
			color: var(--ink);
			text-decoration: underline;
		}
	}
	.trust {
		display: inline-flex;
		align-items: center;
		gap: var(--space-1);
		font-size: var(--fs-sm);
		color: var(--ink-faint);
	}

	.prepaid-note {
		font-size: var(--fs-sm);
		color: var(--ink-faint);
		line-height: var(--lh-normal);
		max-width: 60ch;
		margin: var(--space-2) 0 0;
	}
</style>
