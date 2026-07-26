<script>
	import Icon from '$lib/ui/Icon.svelte';
	import { isSettled } from '$lib/billing.js';
	import { FrontendClient } from '$lib/frontendClient.js';
	import { onMount } from 'svelte';
	import { browser } from '$app/environment';
	import { toast } from '$lib/toast.js';
	import { createQuery } from '@tanstack/svelte-query';
	import { loadRazorpay } from '$lib/loadRazorpay.js';
	import { PRICES, SYMBOL, formatPrice, formatMinor, currencyForCountry } from '$lib/pricing.js';

	let { data } = $props();

	// --- current plan --------------------------------------------------------
	let planName = $derived(data.user?.subscription?.name || 'Free');
	let isPro = $derived(planName === 'Pro');
	let isPlus = $derived(planName === 'Plus');
	let isPaid = $derived(isPro || isPlus);
	let expiresOn = $derived(
		data.user?.subscription?.expires_on
			? new Date(data.user.subscription.expires_on).toLocaleDateString(undefined, {
					day: 'numeric',
					month: 'short',
					year: 'numeric'
				})
			: null
	);

	let planNote = $derived(
		isPaid && expiresOn
			? `Active until ${expiresOn}. Renew whenever you like, we never charge you automatically.`
			: 'Everything is unlocked. Upgrade only when you want more space.'
	);

	// --- controls ------------------------------------------------------------
	let currency = $state('USD');
	let cycle = $state('monthly');
	let annual = $derived(cycle === 'annual');
	const gateway = 'razorpay';

	onMount(() => {
		currency = currencyForCountry(data.user?.country);
	});

	const price = (id, c) => formatPrice(currency, PRICES[currency][id][c]);
	const rawPrice = (id, c) => PRICES[currency][id][c];

	// --- usage ---------------------------------------------------------------
	const storageQuery = createQuery(() => ({
		queryKey: ['fetchStorageStats'],
		queryFn: async () => {
			const { data: d } = await FrontendClient.get('/api/v1/sanctum/user/storage');
			return d?.success || { used: 0, total: 0 };
		},
		enabled: browser
	}));

	let usage = $derived({
		used: storageQuery.data?.used || 0,
		total: storageQuery.data?.total || data.user?.totalAvailableSpace || 0
	});
	let usedPct = $derived(usage.total ? Math.min((usage.used / usage.total) * 100, 100) : 0);

	function fmtSize(bytes) {
		if (!bytes) return '0 B';
		const k = 1024;
		const s = ['B', 'KB', 'MB', 'GB', 'TB'];
		const i = Math.min(Math.floor(Math.log(bytes) / Math.log(k)), s.length - 1);
		return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + s[i];
	}

	// --- promo ---------------------------------------------------------------
	let promoCode = $state('');
	let discount = $state(0);
	let promoApplied = $state(false);
	let verifiedCode = $state('');
	let validatingPromo = $state(false);

	async function applyPromo() {
		if (!promoCode || validatingPromo) return;
		validatingPromo = true;
		try {
			const res = await FrontendClient.post('/api/v1/billing/check-promo', { code: promoCode });
			if (res.data.status === 200 && res.data.data.valid) {
				discount = res.data.data.discount_percentage;
				promoApplied = true;
				verifiedCode = promoCode.toUpperCase();
				toast.success('Promo applied', `${discount}% off your next payment.`);
			} else {
				toast.error('That code is not valid', 'Check the spelling, or it may have expired.');
			}
		} catch (e) {
			toast.error('Could not check that code', e.response?.data?.message || 'Try again in a moment.');
		} finally {
			validatingPromo = false;
		}
	}

	function removePromo() {
		promoApplied = false;
		discount = 0;
		verifiedCode = '';
		promoCode = '';
	}

	// --- plans ---------------------------------------------------------------
	let plans = $derived([
		{
			id: 'free',
			name: 'Free',
			price: `${SYMBOL[currency]}0`,
			per: '/mo',
			billNote: 'free forever',
			tagline: 'Everything, 10 GB of space.',
			features: ['10 GB encrypted storage', 'End-to-end encryption', 'Password + expiring links']
		},
		{
			id: 'plus',
			name: 'Plus',
			price: annual ? price('plus', 'annual') : price('plus', 'monthly'),
			per: annual ? '/yr' : '/mo',
			billNote: annual ? 'billed annually · 2 months free' : `or ${price('plus', 'annual')}/yr`,
			tagline: '20× the space.',
			features: ['200 GB encrypted storage', 'Everything in Free', 'Email support']
		},
		{
			id: 'pro',
			name: 'Pro',
			price: annual ? price('pro', 'annual') : price('pro', 'monthly'),
			per: annual ? '/yr' : '/mo',
			billNote: annual ? 'billed annually · 2 months free' : `or ${price('pro', 'annual')}/yr`,
			tagline: 'Room for everything.',
			features: ['2 TB encrypted storage', 'Everything in Free', 'Priority support']
		}
	]);

	/** Card state: current plan is disabled and badged; lower tiers read "Included". */
	function ctaFor(id) {
		if (id === 'free') {
			return isPaid
				? { label: 'Included', disabled: true, style: 'ghost', badge: null }
				: { label: 'Current plan', disabled: true, style: 'ghost', badge: { text: 'Current', tone: 'ok' } };
		}
		if (id === 'plus') {
			if (isPlus)
				return { label: 'Current plan', disabled: true, style: 'ghost', badge: { text: 'Current', tone: 'ok' } };
			if (isPro) return { label: 'Included', disabled: true, style: 'ghost', badge: null };
			return { label: 'Choose Plus', disabled: false, style: 'ghost', badge: null };
		}
		if (isPro)
			return { label: 'Current plan', disabled: true, style: 'ghost', badge: { text: 'Current', tone: 'ok' } };
		return {
			label: isPlus ? 'Upgrade to Pro' : 'Go Pro',
			disabled: false,
			style: 'solid',
			badge: { text: 'Recommended', tone: 'accent' }
		};
	}

	// --- checkout ------------------------------------------------------------
	let checkoutPlan = $state(null); // 'plus' | 'pro'
	let stage = $state('summary'); // summary | processing | success

	let ckBase = $derived(checkoutPlan ? rawPrice(checkoutPlan, cycle) : 0);
	let ckDiscount = $derived(promoApplied ? Math.round(ckBase * (discount / 100)) : 0);
	let ckTotal = $derived(Math.max(0, ckBase - ckDiscount));

	function openCheckout(id) {
		checkoutPlan = id;
		stage = 'summary';
	}

	function closeCheckout() {
		if (stage === 'processing') return;
		checkoutPlan = null;
		stage = 'summary';
	}

	async function pay() {
		if (!checkoutPlan) return;
		stage = 'processing';
		try {
			const orderRes = await FrontendClient.post('/api/v1/billing/order', {
				order_type: 'plan',
				identifier: checkoutPlan,
				currency,
				gateway,
				cycle,
				promo_code: promoApplied ? verifiedCode : null
			});

			// A 100% promo settles server-side with no gateway round trip.
			if (orderRes.data.success && orderRes.data.success.amount === 0) {
				stage = 'success';
				return;
			}

			const { order_id, amount, currency: cur, key_id } = orderRes.data.success;
			const Razorpay = await loadRazorpay();
			const rzp = new Razorpay({
				key: key_id,
				amount,
				currency: cur,
				name: 'Silocat',
				description: `${checkoutPlan} (${cycle})`,
				order_id,
				handler: async (response) => {
					try {
						await FrontendClient.post('/api/v1/billing/verify', {
							order_id: response.razorpay_order_id,
							payment_id: response.razorpay_payment_id,
							signature: response.razorpay_signature
						});
						stage = 'success';
						loadHistory();
					} catch {
						stage = 'summary';
						toast.error('We could not verify that payment', 'Contact support and we will sort it out.');
					}
				},
				prefill: { email: data.user?.email },
				theme: { color: '#ff4655' },
				modal: { ondismiss: () => (stage = 'summary') }
			});
			rzp.open();
		} catch (e) {
			stage = 'summary';
			toast.error('Could not start checkout', e.response?.data?.message || 'Try again in a moment.');
		}
	}

	let ckPlanLabel = $derived(
		checkoutPlan ? `${checkoutPlan === 'pro' ? 'Pro' : 'Plus'} · ${annual ? 'annual' : 'monthly'}` : ''
	);

	// --- order history -------------------------------------------------------
	let orders = $state([]);
	let historyLoading = $state(true);

	onMount(loadHistory);

	async function loadHistory() {
		historyLoading = true;
		try {
			const res = await FrontendClient.get('/api/v1/billing/history');
			orders = res.data?.success?.orders || res.data?.data?.orders || [];
		} catch (e) {
			console.error('billing history', e);
			orders = [];
		} finally {
			historyLoading = false;
		}
	}

	function statusTone(status) {
		const s = (status || '').toLowerCase();
		if (isSettled(s)) return 'ok';
		if (s === 'failed' || s === 'refunded') return 'danger';
		return 'warn';
	}

	function fmtDate(v) {
		if (!v) return '-';
		return new Date(v).toLocaleDateString(undefined, {
			day: 'numeric',
			month: 'short',
			year: 'numeric'
		});
	}
</script>

<div class="billing">
	<header class="head">
		<h1>Billing</h1>
		<span class="sub">Your plan, storage, and payment history. Nothing auto-renews, ever.</span>
	</header>

	<!-- Current plan -->
	<section class="strip">
		<div class="strip-left">
			<span class="eyebrow">Current plan</span>
			<div class="plan-row">
				<span class="plan-name">{planName}</span>
				{#if isPaid}<span class="badge ok">Active</span>{/if}
			</div>
			<span class="plan-note">{planNote}</span>
		</div>
		<div class="strip-usage">
			<div class="usage-top">
				<span class="mono">{fmtSize(usage.used)}</span>
				<span class="mono faint">of {fmtSize(usage.total)}</span>
			</div>
			<div class="meter">
				<div class="fill" class:warn={usedPct > 90} style="width:{usedPct}%"></div>
			</div>
		</div>
	</section>

	<!-- Change plan -->
	<section class="block">
		<div class="block-head">
			<span class="section-label">Change plan</span>
			<div class="segs">
				<div class="seg">
					<button type="button" class:on={!annual} onclick={() => (cycle = 'monthly')}>Monthly</button>
					<button type="button" class:on={annual} onclick={() => (cycle = 'annual')}>
						Annual · save 17%
					</button>
				</div>
				<div class="seg mono">
					{#each ['USD', 'EUR', 'INR'] as c (c)}
						<button type="button" class:on={currency === c} onclick={() => (currency = c)}>{c}</button>
					{/each}
				</div>
			</div>
		</div>

		<div class="plans">
			{#each plans as p (p.id)}
				{@const cta = ctaFor(p.id)}
				<div class="plan" class:strong={cta.badge?.tone === 'accent'}>
					<div class="plan-top">
						<span class="plan-title">{p.name}</span>
						{#if cta.badge}
							<span class="badge {cta.badge.tone}">{cta.badge.text}</span>
						{/if}
					</div>
					<div class="price-block">
						<div class="price-row">
							<span class="amount">{p.price}</span>
							<span class="per">{p.per}</span>
						</div>
						<span class="bill-note">{p.billNote}</span>
					</div>
					<span class="tagline">{p.tagline}</span>
					<div class="features">
						{#each p.features as f (f)}
							<div class="feature">
								<Icon name="check" size={13} stroke={2.2} />
								<span>{f}</span>
							</div>
						{/each}
					</div>
					<button
						type="button"
						class="plan-cta {cta.style}"
						disabled={cta.disabled}
						onclick={() => openCheckout(p.id)}
					>
						{cta.label}
					</button>
				</div>
			{/each}
		</div>

		<div class="promo-row">
			{#if promoApplied}
				<div class="promo-chip">
					<span class="promo-code">{verifiedCode}</span>
					<span class="promo-off">· {discount}% off</span>
					<button type="button" onclick={removePromo}>Remove</button>
				</div>
			{:else}
				<div class="promo-input">
					<input type="text" placeholder="Promo code" bind:value={promoCode} spellcheck="false" />
					<button type="button" disabled={!promoCode || validatingPromo} onclick={applyPromo}>
						{validatingPromo ? 'Checking…' : 'Apply'}
					</button>
				</div>
			{/if}
			<div class="trust">
				<Icon name="shield" size={14} />
				Payments secured by Razorpay · prepaid, no stored card, no auto-renewal
			</div>
		</div>
	</section>

	<!-- Order history -->
	<section class="block">
		<span class="section-label pad">Order history</span>

		{#if historyLoading}
			<div class="table">
				{#each Array(3) as _, i (i)}
					<div class="orow"><span class="sk" style="width:{180 + i * 40}px"></span></div>
				{/each}
			</div>
		{:else if orders.length === 0}
			<div class="no-orders">
				<span class="no-orders-title">No orders yet</span>
				<span class="no-orders-line">When you buy a plan, the receipt lands here.</span>
			</div>
		{:else}
			<div class="table">
				<div class="ohead">
					<span>Order</span><span>Date</span><span>Amount</span><span>Status</span><span></span>
				</div>
				<!-- Orders are keyed by `reference_id`: the `orders` table has no `id`
				     column, and the invoice number is only assigned once paid. -->
				{#each orders as o (o.reference_id)}
					<div class="orow">
						<span class="oid mono">{o.invoice_number || o.reference_id}</span>
						<span class="mono faint">{fmtDate(o.created_on)}</span>
						<span class="mono">{formatMinor(o.currency, o.amount)}</span>
						<span>
							<span class="badge {statusTone(o.status)}">{o.status}</span>
						</span>
						<!-- No receipt exists until the payment settles, so an unpaid
						     order gets a placeholder rather than a link to a blank invoice. -->
						{#if isSettled(o.status)}
							<a
								class="odl"
								href={`/home/billing/invoice/${o.reference_id}`}
								aria-label="View invoice"
								title="View invoice"
							>
								<Icon name="download" size={15} />
							</a>
						{:else}
							<span class="odl-none" aria-hidden="true"></span>
						{/if}
					</div>
				{/each}
			</div>
		{/if}

		<span class="footnote">
			Plans are prepaid for the term you pick. No card is stored and nothing auto-renews, you're
			never charged without clicking. We'll remind you before your plan ends.
		</span>
	</section>
</div>

<!-- Checkout -->
{#if checkoutPlan}
	<div class="ck-scrim" role="presentation" onclick={closeCheckout}></div>
	<div class="ck-holder" role="dialog" aria-modal="true" aria-label="Checkout">
		<div class="ck">
			{#if stage === 'summary'}
				<div class="ck-head">
					<span class="ck-title">Confirm your plan</span>
					<button type="button" class="ck-x" aria-label="Close" onclick={closeCheckout}>
						<Icon name="close" size={15} />
					</button>
				</div>
				<div class="ck-body">
					<div class="ck-lines">
						<div class="ck-line">
							<span>{ckPlanLabel}</span>
							<span class="mono">{formatPrice(currency, ckBase)}</span>
						</div>
						{#if promoApplied}
							<div class="ck-line">
								<span class="ok-text">{verifiedCode} · {discount}% off</span>
								<span class="mono ok-text">−{formatPrice(currency, ckDiscount)}</span>
							</div>
						{/if}
						<div class="ck-line total">
							<span>Due today</span>
							<span class="mono">{formatPrice(currency, ckTotal)}</span>
						</div>
					</div>

					<div class="ck-pay">
						<span class="ck-label">Pay with</span>
						<div class="ck-method on">
							<span class="radio"><span class="radio-dot"></span></span>
							<div class="method-text">
								<span class="method-name">Razorpay</span>
								<span class="method-sub">UPI · cards · netbanking · wallets</span>
							</div>
						</div>
						<div class="ck-method disabled">
							<span class="radio ghost"></span>
							<span class="method-sub">More payment providers coming soon</span>
						</div>
					</div>

					<div class="ck-trust">
						{#each ['Prepaid for the term you pick', 'No card is stored, nothing auto-renews', '14-day refund, no questions asked'] as t (t)}
							<div class="ck-trust-row">
								<Icon name="check" size={13} stroke={2.2} />
								<span>{t}</span>
							</div>
						{/each}
					</div>
				</div>
				<div class="ck-foot">
					<button type="button" class="ck-cancel" onclick={closeCheckout}>Cancel</button>
					<button type="button" class="ck-pay-btn" onclick={pay}>
						Pay {formatPrice(currency, ckTotal)} with Razorpay
					</button>
				</div>
			{:else if stage === 'processing'}
				<div class="ck-state">
					<Icon name="spinner" size={28} />
					<span class="ck-state-title">Opening Razorpay…</span>
					<span class="ck-state-line">Finish the payment in the window that just opened.</span>
				</div>
			{:else}
				<div class="ck-state">
					<span class="ok-circle"><Icon name="check" size={22} stroke={2.2} /></span>
					<span class="ck-state-title">
						{checkoutPlan === 'pro' ? 'Pro' : 'Plus'} is active
					</span>
					<span class="ck-state-line">
						{expiresOn ? `Active until ${expiresOn}.` : 'Your new space is available right now.'}
					</span>
					<a class="ck-cta" href="/home/files">Start uploading</a>
				</div>
			{/if}
		</div>
	</div>
{/if}

<style lang="scss">
	.billing {
		display: flex;
		flex-direction: column;
		gap: 1.25rem;
		padding-bottom: var(--space-6);
	}

	.head {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
		padding: var(--space-2) 0.125rem 0;

		h1 {
			margin: 0;
			font-size: var(--fs-h2);
			font-weight: var(--fw-black);
			letter-spacing: var(--tracking-tight);
			line-height: var(--lh-tight);
		}
	}

	.sub {
		font-size: var(--fs-sm);
		color: var(--ink-faint);
	}

	/* ---- current plan strip ---- */
	.strip {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-6);
		padding: 1rem 1.25rem;
		border: 1px solid var(--edge);
		border-radius: var(--radius-md);
		background: var(--surface);
	}

	.strip-left {
		display: flex;
		flex-direction: column;
		gap: 0.375rem;
		min-width: 0;
	}

	.eyebrow,
	.section-label {
		font-size: var(--fs-xs);
		font-weight: var(--fw-medium);
		color: var(--ink-faint);
		text-transform: uppercase;
		letter-spacing: 0.06em;
	}

	.plan-row {
		display: flex;
		align-items: center;
		gap: var(--space-2);
	}

	.plan-name {
		font-size: 1.25rem;
		font-weight: var(--fw-semibold);
		letter-spacing: var(--tracking-tight);
	}

	.plan-note {
		font-size: var(--fs-sm);
		color: var(--ink-mute);
	}

	.strip-usage {
		flex: 0 1 320px;
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}

	.usage-top {
		display: flex;
		align-items: baseline;
		justify-content: space-between;
	}

	.meter {
		height: 6px;
		border-radius: var(--radius-full);
		background: var(--tint-softer);
		overflow: hidden;
	}

	.fill {
		height: 100%;
		border-radius: var(--radius-full);
		background: var(--accent);

		&.warn {
			background: var(--warn);
		}
	}

	.mono {
		font-family: var(--font-mono);
		font-size: var(--fs-sm);
	}

	.faint {
		color: var(--ink-faint);
	}

	/* ---- blocks ---- */
	.block {
		display: flex;
		flex-direction: column;
		gap: 0.875rem;
	}

	.block-head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-4);
		padding-inline: 0.125rem;
	}

	.pad {
		padding-inline: 0.125rem;
	}

	.segs {
		display: flex;
		align-items: center;
		gap: var(--space-3);
	}

	.seg {
		display: flex;
		padding: 2px;
		border-radius: 8px;
		background: var(--tint-soft);
		border: 1px solid var(--edge);

		button {
			height: 26px;
			padding-inline: 0.75rem;
			border: 1px solid transparent;
			background: transparent;
			border-radius: var(--radius-sm);
			font: inherit;
			font-size: var(--fs-xs);
			font-weight: var(--fw-medium);
			color: var(--ink-faint);
			cursor: pointer;
			white-space: nowrap;
			transition:
				background var(--dur-fast) var(--ease),
				color var(--dur-fast) var(--ease);

			&.on {
				background: var(--raised);
				border-color: var(--edge);
				color: var(--ink);
			}
		}

		&.mono button {
			font-family: var(--font-mono);
			padding-inline: 0.625rem;
			font-weight: var(--fw-regular);
		}
	}

	/* ---- plan cards ---- */
	.plans {
		display: grid;
		grid-template-columns: repeat(3, 1fr);
		gap: 0.75rem;
	}

	.plan {
		display: flex;
		flex-direction: column;
		gap: 0.875rem;
		padding: 1.25rem;
		border-radius: var(--radius-md);
		background: var(--surface);
		border: 1px solid var(--edge);

		&.strong {
			border-color: var(--edge-strong);
		}
	}

	.plan-top {
		display: flex;
		align-items: center;
		justify-content: space-between;
	}

	.plan-title {
		font-size: 0.9375rem;
		font-weight: var(--fw-semibold);
		letter-spacing: var(--tracking-tight);
	}

	.badge {
		display: inline-flex;
		align-items: center;
		height: 20px;
		padding-inline: 0.4375rem;
		border-radius: var(--radius-sm);
		font-size: var(--fs-xs);
		font-weight: var(--fw-medium);

		&.ok {
			background: var(--ok-soft);
			color: var(--ok);
		}
		&.accent {
			background: var(--accent-soft);
			color: var(--accent);
		}
		&.warn {
			background: var(--warn-soft);
			color: var(--warn);
		}
		&.danger {
			background: var(--danger-soft);
			color: var(--danger);
		}
	}

	.price-block {
		display: flex;
		flex-direction: column;
		gap: 0.125rem;
	}

	.price-row {
		display: flex;
		align-items: baseline;
		gap: 0.375rem;
	}

	.amount {
		font-family: var(--font-mono);
		font-size: 1.75rem;
		font-weight: var(--fw-medium);
		letter-spacing: var(--tracking-tight);
	}

	.per,
	.bill-note {
		font-family: var(--font-mono);
		color: var(--ink-faint);
	}
	.per {
		font-size: var(--fs-sm);
	}
	.bill-note {
		font-size: var(--fs-xs);
	}

	.tagline {
		font-size: var(--fs-sm);
		color: var(--ink-mute);
	}

	.features {
		display: flex;
		flex-direction: column;
		gap: 0.4375rem;
		padding-top: 0.125rem;
	}

	.feature {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		color: var(--ink-faint);

		span {
			font-size: var(--fs-sm);
			color: var(--ink-mute);
		}
	}

	.plan-cta {
		margin-top: auto;
		height: 36px;
		border-radius: var(--radius-md);
		border: 1px solid var(--edge);
		background: none;
		font: inherit;
		font-size: var(--fs-sm);
		font-weight: var(--fw-medium);
		color: var(--ink);
		cursor: pointer;
		transition: filter var(--dur-fast) var(--ease);

		&.solid {
			background: var(--accent);
			border-color: transparent;
			color: #fff;
		}
		&:hover:not(:disabled) {
			filter: brightness(1.08);
		}
		&:disabled {
			opacity: 0.55;
			cursor: default;
		}
	}

	/* ---- promo ---- */
	.promo-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-4);
		padding-inline: 0.125rem;
		flex-wrap: wrap;
	}

	.promo-input {
		display: flex;
		gap: 0.375rem;

		input {
			width: 150px;
			height: 32px;
			padding-inline: 0.625rem;
			border-radius: var(--radius-sm);
			background: var(--surface);
			border: 1px solid var(--edge);
			color: var(--ink);
			font-family: var(--font-mono);
			font-size: var(--fs-sm);
			outline: none;

			&:focus {
				border-color: var(--accent);
				box-shadow: 0 0 0 3px var(--focus-ring);
			}
		}

		button {
			height: 32px;
			padding-inline: 0.75rem;
			border-radius: var(--radius-sm);
			border: 1px solid var(--edge);
			background: none;
			font: inherit;
			font-size: var(--fs-sm);
			font-weight: var(--fw-medium);
			color: var(--ink);
			cursor: pointer;

			&:hover:not(:disabled) {
				background: var(--tint-soft);
			}
			&:disabled {
				opacity: 0.5;
				cursor: not-allowed;
			}
		}
	}

	.promo-chip {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		height: 32px;
		padding: 0 0.375rem 0 0.625rem;
		border-radius: 8px;
		background: var(--ok-soft);
		border: 1px solid var(--edge);

		button {
			border: 0;
			background: none;
			font: inherit;
			font-size: var(--fs-xs);
			color: var(--ink-faint);
			padding: 0.125rem 0.375rem;
			border-radius: 5px;
			cursor: pointer;

			&:hover {
				background: var(--tint-softer);
				color: var(--ink);
			}
		}
	}

	.promo-code {
		font-family: var(--font-mono);
		font-size: var(--fs-xs);
		font-weight: var(--fw-semibold);
		color: var(--ok);
	}

	.promo-off {
		font-size: var(--fs-xs);
		color: var(--ink-mute);
	}

	.trust {
		display: flex;
		align-items: center;
		gap: 0.4375rem;
		font-size: var(--fs-xs);
		color: var(--ink-faint);
	}

	/* ---- order history ---- */
	.table {
		border: 1px solid var(--edge);
		border-radius: var(--radius-md);
		background: var(--surface);
		overflow: hidden;
	}

	.ohead,
	.orow {
		display: grid;
		grid-template-columns: 1.4fr 1fr 1fr 0.8fr 48px;
		gap: var(--space-4);
		align-items: center;
		padding: 0.625rem 1rem;
		border-bottom: 1px solid var(--edge);
	}

	.ohead {
		font-size: var(--fs-xs);
		color: var(--ink-faint);
	}

	.orow {
		transition: background var(--dur-fast) var(--ease);

		&:last-child {
			border-bottom: 0;
		}
		&:hover {
			background: var(--surface-hover);
		}
	}

	.oid {
		font-size: var(--fs-xs);
		color: var(--ink-mute);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.odl-none {
		width: 28px;
		height: 28px;
		justify-self: end;
	}

	.odl {
		width: 28px;
		height: 28px;
		border-radius: var(--radius-sm);
		display: grid;
		place-items: center;
		color: var(--ink-faint);
		justify-self: end;
		transition:
			background var(--dur-fast) var(--ease),
			color var(--dur-fast) var(--ease);

		&:hover {
			background: var(--tint-softer);
			color: var(--ink);
		}
	}

	.no-orders {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: var(--space-2);
		border: 1px solid var(--edge);
		border-radius: var(--radius-md);
		background: var(--surface);
		padding: 2.5rem 1rem;
		text-align: center;
	}

	.no-orders-title {
		font-size: 0.9375rem;
		font-weight: var(--fw-medium);
	}

	.no-orders-line {
		font-size: var(--fs-sm);
		color: var(--ink-mute);
	}

	.footnote {
		font-size: var(--fs-xs);
		color: var(--ink-faint);
		padding-inline: 0.125rem;
	}

	.sk {
		display: block;
		height: 0.9rem;
		border-radius: var(--radius-sm);
		background: var(--tint-softer);
	}

	/* ---- checkout ---- */
	.ck-scrim {
		position: fixed;
		inset: 0;
		background: var(--scrim);
		z-index: 1000;
	}

	.ck-holder {
		position: fixed;
		inset: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: var(--space-5);
		z-index: 1001;
		pointer-events: none;
	}

	.ck {
		width: 100%;
		max-width: 440px;
		display: flex;
		flex-direction: column;
		background: var(--raised);
		border: 1px solid var(--edge);
		border-radius: var(--radius-lg);
		box-shadow: var(--shadow-overlay);
		overflow: hidden;
		pointer-events: auto;
	}

	.ck-head {
		display: flex;
		align-items: center;
		gap: 0.625rem;
		padding: 1rem 1rem 0.875rem;
	}

	.ck-title {
		flex: 1;
		font-size: 0.9375rem;
		font-weight: var(--fw-semibold);
		letter-spacing: var(--tracking-tight);
	}

	.ck-x {
		width: 28px;
		height: 28px;
		border: 0;
		background: none;
		border-radius: var(--radius-sm);
		display: grid;
		place-items: center;
		color: var(--ink-faint);
		cursor: pointer;

		&:hover {
			background: var(--tint-soft);
			color: var(--ink);
		}
	}

	.ck-body {
		display: flex;
		flex-direction: column;
		gap: 0.875rem;
		padding: 0 1rem 1rem;
	}

	.ck-lines {
		display: flex;
		flex-direction: column;
		border: 1px solid var(--edge);
		border-radius: var(--radius-md);
		overflow: hidden;
	}

	.ck-line {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 0.625rem 0.875rem;
		border-bottom: 1px solid var(--edge);
		font-size: var(--fs-sm);
		color: var(--ink-mute);

		&:last-child {
			border-bottom: 0;
		}
		&.total {
			background: var(--tint-soft);
			color: var(--ink);
			font-weight: var(--fw-medium);
		}
	}

	.ok-text {
		color: var(--ok);
	}

	.ck-pay {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}

	.ck-label {
		font-size: var(--fs-xs);
		color: var(--ink-faint);
	}

	.ck-method {
		display: flex;
		align-items: center;
		gap: 0.625rem;
		padding: 0.625rem 0.75rem;
		border: 1px solid var(--edge);
		border-radius: var(--radius-md);

		&.on {
			border-color: var(--accent);
			background: var(--accent-soft);
		}
		&.disabled {
			border-style: dashed;
			opacity: 0.6;
		}
	}

	.radio {
		flex: 0 0 auto;
		width: 16px;
		height: 16px;
		border-radius: var(--radius-full);
		border: 1px solid var(--accent);
		display: grid;
		place-items: center;

		&.ghost {
			border-color: var(--edge-strong);
		}
	}

	.radio-dot {
		width: 8px;
		height: 8px;
		border-radius: var(--radius-full);
		background: var(--accent);
	}

	.method-text {
		display: flex;
		flex-direction: column;
	}

	.method-name {
		font-size: var(--fs-sm);
		font-weight: var(--fw-medium);
	}

	.method-sub {
		font-size: var(--fs-xs);
		color: var(--ink-faint);
	}

	.ck-trust {
		display: flex;
		flex-direction: column;
		gap: 0.375rem;
	}

	.ck-trust-row {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		color: var(--ok);

		span {
			font-size: var(--fs-xs);
			color: var(--ink-mute);
		}
	}

	.ck-foot {
		display: flex;
		flex-direction: column-reverse;
		gap: var(--space-2);
		padding: 0.875rem 1rem;
		border-top: 1px solid var(--edge);
	}

	.ck-cancel {
		height: 34px;
		border: 0;
		background: none;
		border-radius: var(--radius-md);
		font: inherit;
		font-size: var(--fs-sm);
		color: var(--ink-mute);
		cursor: pointer;

		&:hover {
			background: var(--tint-soft);
			color: var(--ink);
		}
	}

	.ck-pay-btn {
		height: 40px;
		border: 0;
		border-radius: var(--radius-md);
		background: var(--accent);
		color: #fff;
		font: inherit;
		font-size: var(--fs-sm);
		font-weight: var(--fw-medium);
		cursor: pointer;
		transition: background var(--dur-fast) var(--ease);

		&:hover {
			background: var(--accent-hover);
		}
	}

	.ck-state {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.75rem;
		padding: 2.5rem 1.5rem;
		text-align: center;
		color: var(--ink-faint);
	}

	.ok-circle {
		display: grid;
		place-items: center;
		width: 48px;
		height: 48px;
		border-radius: var(--radius-full);
		background: var(--ok-soft);
		color: var(--ok);
	}

	.ck-state-title {
		font-size: var(--fs-lg);
		font-weight: var(--fw-semibold);
		letter-spacing: var(--tracking-tight);
		color: var(--ink);
	}

	.ck-state-line {
		font-size: var(--fs-sm);
		color: var(--ink-mute);
	}

	.ck-cta {
		margin-top: var(--space-2);
		display: inline-flex;
		align-items: center;
		height: 36px;
		padding-inline: 1rem;
		border-radius: var(--radius-md);
		background: var(--accent);
		color: #fff;
		font-size: var(--fs-sm);
		font-weight: var(--fw-medium);
		text-decoration: none;

		&:hover {
			background: var(--accent-hover);
			color: #fff;
		}
	}

	@media (max-width: 980px) {
		.plans {
			grid-template-columns: 1fr;
		}
		.strip {
			flex-direction: column;
			align-items: stretch;
			gap: var(--space-4);
		}
		.block-head {
			flex-direction: column;
			align-items: flex-start;
			gap: var(--space-2);
		}
		.ohead,
		.orow {
			grid-template-columns: 1.4fr 1fr 0.8fr 48px;
		}
		.ohead span:nth-child(2),
		.orow > .mono.faint {
			display: none;
		}
	}
</style>
