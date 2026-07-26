<script>
	import Icon from '$lib/ui/Icon.svelte';
	import Seo from '$lib/components/Seo.svelte';
	import { softwareApplicationSchema, breadcrumbSchema } from '$lib/seo.js';
	import Navbar from '$lib/components/Navbar.svelte';
	import Footer from '$lib/components/Footer.svelte';
	import { PRICES, SYMBOL, formatPrice } from '$lib/pricing.js';

	let currency = $state('USD');
	let cycle = $state('monthly');

	let annual = $derived(cycle === 'annual');
	let symbol = $derived(SYMBOL[currency]);

	const price = (id, c) => formatPrice(currency, PRICES[currency][id][c]);

	let plans = $derived([
		{
			id: 'free',
			name: 'Free',
			price: `${symbol}0`,
			per: '/mo',
			billNote: 'free forever',
			tagline: 'Everything, 10 GB of space.',
			badge: '',
			features: [
				'10 GB encrypted storage',
				'End-to-end encryption',
				'Password + expiring share links',
				'Up to 20 GB anonymous drops'
			],
			cta: 'Get started',
			href: '/auth/signup',
			style: 'ghost'
		},
		{
			id: 'plus',
			name: 'Plus',
			price: annual ? price('plus', 'annual') : price('plus', 'monthly'),
			per: annual ? '/yr' : '/mo',
			billNote: annual ? 'billed annually · 2 months free' : `or ${price('plus', 'annual')}/yr`,
			tagline: '20× the space.',
			badge: 'Recommended',
			features: ['200 GB encrypted storage', 'Everything in Free', 'Email support'],
			cta: 'Choose Plus',
			href: '/auth/signup?plan=plus',
			style: 'ghost',
			strong: true
		},
		{
			id: 'pro',
			name: 'Pro',
			price: annual ? price('pro', 'annual') : price('pro', 'monthly'),
			per: annual ? '/yr' : '/mo',
			billNote: annual ? 'billed annually · 2 months free' : `or ${price('pro', 'annual')}/yr`,
			tagline: 'Room for everything.',
			badge: '',
			features: ['2 TB encrypted storage', 'Everything in Free', 'Priority support'],
			cta: 'Go Pro',
			href: '/auth/signup?plan=pro',
			style: 'solid'
		}
	]);
</script>

<Seo
	title="Pricing: Simple, transparent, secure | Silocat"
	description="Silocat plans for anonymous and power users. Zero-knowledge end-to-end encrypted file sharing and storage, with a free tier, affordable upgrades, and a free self-host option."
	schema={[
		softwareApplicationSchema(),
		breadcrumbSchema([
			{ name: 'Home', path: '/' },
			{ name: 'Pricing', path: '/pricing' }
		])
	]}
/>

<div class="page">
	<Navbar />

	<main class="main">
		<section class="head">
			<h1>Pricing</h1>
			<p class="sub">Every feature is free. You only pay for space.</p>

			<div class="controls">
				<div class="seg">
					<button
						type="button"
						class:on={!annual}
						onclick={() => (cycle = 'monthly')}
						aria-pressed={!annual}
					>
						Monthly
					</button>
					<button
						type="button"
						class:on={annual}
						onclick={() => (cycle = 'annual')}
						aria-pressed={annual}
					>
						Annual · save 17%
					</button>
				</div>

				<div class="seg mono">
					{#each ['USD', 'EUR', 'INR'] as c (c)}
						<button
							type="button"
							class:on={currency === c}
							onclick={() => (currency = c)}
							aria-pressed={currency === c}
						>
							{c}
						</button>
					{/each}
				</div>
			</div>
		</section>

		<section class="grid">
			{#each plans as p (p.id)}
				<div class="plan" class:strong={p.strong}>
					<div class="plan-top">
						<span class="plan-name">{p.name}</span>
						{#if p.badge}<span class="badge">{p.badge}</span>{/if}
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

					<a href={p.href} class="cta {p.style}">{p.cta}</a>
				</div>
			{/each}
		</section>

		<section class="tail">
			<div class="selfhost">
				<div class="sh-text">
					<span class="sh-title">Run Silocat on your own hardware.</span>
					<span class="sh-sub">Free forever, AGPL-3.0, no limits.</span>
				</div>
				<a
					href="https://github.com/clickswave/silocat"
					target="_blank"
					rel="noreferrer"
					class="sh-btn"
				>
					<Icon name="github" size={15} />
					View on GitHub
				</a>
			</div>

			<p class="footnote">
				Plans are prepaid, no stored card and no auto-renewal. Anonymous drops are always free and
				expire after 7 days; account storage stays until you delete it.
			</p>
		</section>
	</main>

	<Footer />
</div>

<style lang="scss">
	.page {
		min-height: 100vh;
		display: flex;
		flex-direction: column;
		background: var(--bg);
		color: var(--ink);
		font-family: var(--font-sans);
		font-size: var(--fs-body);
		line-height: var(--lh-normal);
	}

	.main {
		flex: 1;
		width: 100%;
		max-width: var(--container);
		margin: 0 auto;
		padding-inline: var(--gutter);
	}

	.head {
		padding: clamp(2.5rem, 7vw, 4.5rem) 0 var(--space-6);
		display: flex;
		flex-direction: column;
		align-items: center;
		text-align: center;
		gap: var(--space-4);

		h1 {
			margin: 0;
			font-size: var(--fs-h1);
			font-weight: var(--fw-black);
			letter-spacing: var(--tracking-tight);
			line-height: var(--lh-tight);
		}
	}

	.sub {
		margin: 0;
		max-width: 48ch;
		font-size: var(--fs-lg);
		color: var(--ink-mute);
	}

	.controls {
		display: flex;
		flex-wrap: wrap;
		justify-content: center;
		gap: var(--space-3);
		padding-top: var(--space-2);
	}

	.seg {
		display: flex;
		padding: 2px;
		border-radius: 8px;
		background: var(--tint-soft);
		border: 1px solid var(--edge);

		button {
			height: 28px;
			padding-inline: 0.875rem;
			border-radius: var(--radius-sm);
			border: 1px solid transparent;
			background: transparent;
			font: inherit;
			font-size: var(--fs-sm);
			font-weight: var(--fw-medium);
			color: var(--ink-faint);
			cursor: pointer;
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
			padding-inline: 0.75rem;
			font-weight: var(--fw-regular);
		}
	}

	.grid {
		display: grid;
		grid-template-columns: repeat(3, 1fr);
		gap: 0.75rem;
		max-width: 960px;
		margin: 0 auto;
	}

	.plan {
		display: flex;
		flex-direction: column;
		gap: 0.875rem;
		padding: var(--space-5);
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

	.plan-name {
		font-size: 1rem;
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
		background: var(--accent-soft);
		color: var(--accent);
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
		font-size: 2rem;
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

	.cta {
		margin-top: auto;
		display: flex;
		align-items: center;
		justify-content: center;
		height: 38px;
		border-radius: var(--radius-md);
		font-size: var(--fs-sm);
		font-weight: var(--fw-medium);
		text-decoration: none;
		transition: filter var(--dur-fast) var(--ease);

		&:hover {
			filter: brightness(1.08);
		}

		&.ghost {
			border: 1px solid var(--edge);
			color: var(--ink);
		}
		&.solid {
			background: var(--accent);
			color: #fff;
			border: 1px solid transparent;
		}
	}

	.tail {
		max-width: 960px;
		margin: 0 auto;
		padding-top: var(--space-6);
	}

	.selfhost {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-4);
		padding: 1.25rem;
		border: 1px solid var(--edge);
		border-radius: var(--radius-md);
		background: var(--surface);
	}

	.sh-text {
		display: flex;
		flex-direction: column;
		gap: 0.125rem;
	}

	.sh-title {
		font-size: 0.9375rem;
		font-weight: var(--fw-semibold);
		letter-spacing: var(--tracking-tight);
	}

	.sh-sub {
		font-size: var(--fs-sm);
		color: var(--ink-mute);
	}

	.sh-btn {
		display: flex;
		align-items: center;
		gap: 0.4375rem;
		height: 36px;
		padding-inline: 1rem;
		border-radius: var(--radius-md);
		border: 1px solid var(--edge);
		color: var(--ink);
		font-size: var(--fs-sm);
		font-weight: var(--fw-medium);
		text-decoration: none;
		transition:
			background var(--dur-fast) var(--ease),
			border-color var(--dur-fast) var(--ease);

		&:hover {
			background: var(--tint-soft);
			border-color: var(--edge-strong);
			color: var(--ink);
		}
	}

	.footnote {
		margin: 0;
		padding: 1.25rem 0.125rem clamp(2.5rem, 6vw, 4rem);
		font-size: var(--fs-xs);
		color: var(--ink-faint);
		text-align: center;
	}

	@media (max-width: 860px) {
		.grid {
			grid-template-columns: 1fr;
		}
	}
</style>
