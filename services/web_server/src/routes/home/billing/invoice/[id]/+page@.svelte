<script>
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import Icon from '$lib/ui/Icon.svelte';
	import { FrontendClient } from '$lib/frontendClient.js';
	import { formatMinor } from '$lib/pricing.js';

	let { data } = $props();

	let order = $state(null);
	let loading = $state(true);
	let notFound = $state(false);

	onMount(async () => {
		try {
			const res = await FrontendClient.get('/api/v1/billing/history');
			const orders = res.data?.success?.orders || res.data?.data?.orders || [];
			order = orders.find((o) => o.reference_id === $page.params.id) || null;
			notFound = !order;
		} catch (e) {
			console.error('invoice load', e);
			notFound = true;
		} finally {
			loading = false;
		}
	});

	const money = (amount, currency) => formatMinor(currency, amount);

	function fmtDate(v) {
		if (!v) return '-';
		return new Date(v).toLocaleDateString(undefined, {
			day: 'numeric',
			month: 'long',
			year: 'numeric'
		});
	}

	let lineLabel = $derived(
		order
			? order.additional_space > 0
				? 'Additional storage'
				: `Silocat ${order.subscription_name} · ${order.subscription_cycle}`
			: ''
	);

	let discountMinor = $derived(Number(order?.details?.discount_amount ?? 0));
	let baseMinor = $derived(Number(order?.amount ?? 0) + discountMinor);
	let promoCode = $derived(order?.details?.promo_code || null);

	function printPage() {
		window.print();
	}
</script>

<svelte:head>
	<title>{order?.invoice_number || 'Invoice'} · Silocat</title>
	<meta name="robots" content="noindex, nofollow" />
</svelte:head>

<!-- A print-owning page: colours are fixed light because this is a document
     people print or save as PDF, not a screen that follows the app theme. -->
<div class="sheet-page">
	<div class="toolbar">
		<a href="/home/billing" class="tb-back">← Billing</a>
		<button type="button" class="tb-print" onclick={printPage}>
			<Icon name="download" size={15} />
			Download PDF
		</button>
	</div>

	{#if loading}
		<div class="sheet centered"><span class="muted">Loading invoice…</span></div>
	{:else if notFound}
		<div class="sheet centered">
			<span class="muted">That invoice could not be found.</span>
			<a href="/home/billing" class="link">Back to billing</a>
		</div>
	{:else}
		<article class="sheet">
			<header class="brand">
				<img src="/silocat-logo.png" alt="" width="34" height="34" />
				<span class="wordmark">silocat</span>
			</header>

			<div class="rule"></div>

			<div class="title-row">
				<h1>Invoice</h1>
				<span class="inv-no">{order.invoice_number || order.reference_id}</span>
			</div>

			<dl class="meta">
				<div><dt>Issued</dt><dd>{fmtDate(order.created_on)}</dd></div>
				<div><dt>Order</dt><dd class="mono">{order.reference_id}</dd></div>
				<div><dt>Payment</dt><dd>{order.payment_gateway}</dd></div>
				<div>
					<dt>Status</dt>
					<dd><span class="paid">{(order.status || 'paid').toUpperCase()}</span></dd>
				</div>
			</dl>

			<div class="parties">
				<div>
					<span class="party-label">Billed to</span>
					<span class="party-name">{data.user?.username || 'Account holder'}</span>
					<span class="party-line">{data.user?.email || ''}</span>
				</div>
				<div>
					<span class="party-label">From</span>
					<span class="party-name">Clickswave Labs Private Limited</span>
					<span class="party-line">Gujarat, India</span>
					<span class="party-line">team@silo.cat</span>
				</div>
			</div>

			<table class="lines">
				<thead>
					<tr><th>Description</th><th class="right">Amount</th></tr>
				</thead>
				<tbody>
					<tr>
						<td>{lineLabel}</td>
						<td class="right mono">{money(baseMinor, order.currency)}</td>
					</tr>
					{#if discountMinor > 0}
						<tr>
							<td>Promotional discount{promoCode ? ` (${promoCode})` : ''}</td>
							<td class="right mono">−{money(discountMinor, order.currency)}</td>
						</tr>
					{/if}
				</tbody>
			</table>

			<div class="totals">
				<div class="total-rule"></div>
				<div class="total-row">
					<span>Total paid</span>
					<span class="mono total-amount">{money(order.amount, order.currency)}</span>
				</div>
			</div>

			<div class="reassure">
				This plan is prepaid. No payment card is stored and nothing auto-renews, so you will not be
				charged again unless you choose to renew.
			</div>

			<footer class="foot">
				Silocat is a Clickswave Labs product · silo.cat · Questions? team@silo.cat
			</footer>
		</article>
	{/if}
</div>

<style lang="scss">
	.sheet-page {
		min-height: 100vh;
		background: #f4f4f5;
		/* Bottom padding clears the floating toolbar, so the last block of the
		   sheet is never sitting underneath it at full scroll. */
		padding: 2.5rem 1rem 7rem;
		display: flex;
		flex-direction: column;
		align-items: center;
		font-family: var(--font-sans);
	}

	.toolbar {
		position: fixed;
		left: 50%;
		bottom: 1.5rem;
		transform: translateX(-50%);
		z-index: 10;
		display: flex;
		align-items: center;
		gap: var(--space-2);
		padding: 0.5rem 0.625rem;
		border-radius: var(--radius-full);
		background: #16161a;
		box-shadow: 0 16px 48px -12px rgba(0, 0, 0, 0.4);
	}

	.tb-back {
		font-size: var(--fs-sm);
		color: rgba(255, 255, 255, 0.7);
		text-decoration: none;
		padding-inline: 0.5rem;

		&:hover {
			color: #fff;
		}
	}

	.tb-print {
		display: inline-flex;
		align-items: center;
		gap: 0.4375rem;
		height: 32px;
		padding-inline: 0.875rem;
		border: 0;
		border-radius: var(--radius-full);
		background: #ff4655;
		color: #fff;
		font: inherit;
		font-size: var(--fs-sm);
		font-weight: var(--fw-medium);
		cursor: pointer;
	}

	/* Letter/A4 sheet. Colours are hard-coded light: this is a document. */
	.sheet {
		width: 100%;
		max-width: 720px;
		min-height: 940px;
		background: #ffffff;
		color: #16161a;
		border-radius: 4px;
		box-shadow: 0 8px 32px -12px rgba(0, 0, 0, 0.25);
		padding: 3.5rem 3rem;
		display: flex;
		flex-direction: column;
		gap: 1.75rem;

		&.centered {
			min-height: 320px;
			align-items: center;
			justify-content: center;
			gap: var(--space-3);
		}
	}

	.muted {
		color: #55555f;
		font-size: var(--fs-body);
	}

	.link {
		color: #e63946;
		font-size: var(--fs-sm);
		text-decoration: none;
	}

	.brand {
		display: flex;
		align-items: center;
		gap: 0.625rem;

		img {
			width: 34px;
			height: 34px;
			border-radius: 9px;
		}
	}

	.wordmark {
		font-size: 1.125rem;
		font-weight: var(--fw-semibold);
		letter-spacing: var(--tracking-tight);
	}

	.rule {
		height: 2px;
		background: #16161a;
	}

	.title-row {
		display: flex;
		align-items: baseline;
		justify-content: space-between;
		gap: var(--space-4);

		h1 {
			margin: 0;
			font-size: 1.75rem;
			font-weight: var(--fw-black);
			letter-spacing: var(--tracking-tight);
			/* Explicit: the global heading colour follows the app theme, and this
			   sheet is always light. */
			color: #16161a;
		}
	}

	.inv-no {
		font-family: var(--font-mono);
		font-size: var(--fs-body);
		color: #55555f;
	}

	.meta {
		display: grid;
		grid-template-columns: repeat(4, 1fr);
		gap: var(--space-4);
		margin: 0;

		div {
			display: flex;
			flex-direction: column;
			gap: 0.25rem;
		}

		dt {
			font-size: 0.6875rem;
			text-transform: uppercase;
			letter-spacing: 0.08em;
			color: #8b8b95;
		}

		dd {
			margin: 0;
			font-size: var(--fs-sm);
		}
	}

	.paid {
		display: inline-flex;
		align-items: center;
		height: 20px;
		padding-inline: 0.4375rem;
		border-radius: var(--radius-sm);
		background: rgba(26, 158, 99, 0.12);
		color: #1a9e63;
		font-size: var(--fs-xs);
		font-weight: var(--fw-semibold);
		letter-spacing: 0.04em;
	}

	.parties {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: var(--space-6);

		div {
			display: flex;
			flex-direction: column;
			gap: 0.25rem;
		}
	}

	.party-label {
		font-size: 0.6875rem;
		text-transform: uppercase;
		letter-spacing: 0.08em;
		color: #8b8b95;
		margin-bottom: 0.125rem;
	}

	.party-name {
		font-size: var(--fs-sm);
		font-weight: var(--fw-semibold);
	}

	.party-line {
		font-size: var(--fs-sm);
		color: #55555f;
	}

	.lines {
		width: 100%;
		border-collapse: collapse;

		th {
			text-align: left;
			padding: 0 0 0.625rem;
			border-bottom: 1px solid rgba(0, 0, 0, 0.12);
			font-size: 0.6875rem;
			text-transform: uppercase;
			letter-spacing: 0.08em;
			color: #8b8b95;
			font-weight: var(--fw-medium);
		}

		td {
			padding: 0.875rem 0;
			border-bottom: 1px solid rgba(0, 0, 0, 0.06);
			font-size: var(--fs-sm);
		}
	}

	.right {
		text-align: right;
	}

	.mono {
		font-family: var(--font-mono);
	}

	.totals {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
		align-items: flex-end;
	}

	.total-rule {
		width: 220px;
		height: 1.5px;
		background: #16161a;
	}

	.total-row {
		display: flex;
		align-items: baseline;
		justify-content: space-between;
		gap: var(--space-6);
		width: 220px;
		font-size: var(--fs-body);
		font-weight: var(--fw-semibold);
	}

	.total-amount {
		font-size: 1.125rem;
	}

	.reassure {
		margin-top: auto;
		padding: 0.875rem 1rem;
		border: 1px solid rgba(0, 0, 0, 0.1);
		border-radius: 8px;
		background: #fafafa;
		font-size: var(--fs-sm);
		color: #55555f;
		line-height: var(--lh-normal);
	}

	.foot {
		font-size: var(--fs-xs);
		color: #8b8b95;
		text-align: center;
	}

	/* A zero page margin is what suppresses the browser's own print header and
	   footer (date, document title, URL, "1/1"). The sheet then supplies the
	   real margin itself, so the printed document has proper white space. */
	@page {
		size: auto;
		margin: 0;
	}

	@media print {
		.toolbar {
			display: none;
		}

		:global(html),
		:global(body) {
			background: #fff;
			/* Without this the sheet's screen height spills onto a second page
			   and the preview shows a scrollbar. */
			height: auto;
			overflow: visible;
		}

		.sheet-page {
			background: #fff;
			padding: 0;
			min-height: 0;
			display: block;
		}

		.sheet {
			box-shadow: none;
			border-radius: 0;
			max-width: none;
			min-height: 0;
			width: 100%;
			padding: 16mm 14mm;
			gap: 1.5rem;
			break-inside: avoid;
		}

		/* The reassurance card is pushed to the bottom on screen by `margin-top:
		   auto`; in print that would strand it on page two. */
		.reassure {
			margin-top: 1.5rem;
		}
	}

	@media (max-width: 640px) {
		.sheet {
			padding: 2rem 1.25rem;
		}
		.meta {
			grid-template-columns: repeat(2, 1fr);
		}
		.parties {
			grid-template-columns: 1fr;
			gap: var(--space-4);
		}
	}
</style>
