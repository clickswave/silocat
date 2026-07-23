<script>
	import Icon from '@iconify/svelte';
	import Seo from '$lib/components/Seo.svelte';
	import { softwareApplicationSchema, breadcrumbSchema } from '$lib/seo.js';
	import Navbar from '$lib/components/Navbar.svelte';
	import Footer from '$lib/components/Footer.svelte';
	import { Button, Segmented, Badge } from '$lib/ui';

	let currency = $state('USD');
	let cycle = $state('monthly');

	const SYMBOL = { USD: '$', EUR: '€', INR: '₹' };
	let symbol = $derived(SYMBOL[currency]);

	const PRICES = {
		USD: { plus: { monthly: 4, annual: 39 }, pro: { monthly: 10, annual: 96 } },
		EUR: { plus: { monthly: 4, annual: 39 }, pro: { monthly: 9, annual: 90 } },
		INR: { plus: { monthly: 349, annual: 3490 }, pro: { monthly: 899, annual: 8990 } }
	};

	function price(id) {
		return PRICES[currency][id][cycle];
	}
	let per = $derived(cycle === 'annual' ? '/yr' : '/mo');

	let plans = $derived([
		{
			id: 'free',
			name: 'Free',
			amount: `${symbol}0`,
			per: '/mo',
			description: 'Everything, 10 GB of space.',
			features: [
				'10 GB encrypted storage',
				'End-to-end encryption',
				'Password + expiring share links',
				'Up to 20 GB anonymous drops'
			],
			cta: 'Get started',
			href: '/auth/signup',
			variant: 'ghost'
		},
		{
			id: 'plus',
			name: 'Plus',
			amount: `${symbol}${price('plus')}`,
			per,
			description: '20× the space.',
			features: ['200 GB encrypted storage', 'Everything in Free', 'Email support'],
			cta: 'Choose Plus',
			href: '/auth/signup',
			variant: 'ghost'
		},
		{
			id: 'pro',
			name: 'Pro',
			amount: `${symbol}${price('pro')}`,
			per,
			description: 'Room for everything.',
			highlight: true,
			features: ['2 TB encrypted storage', 'Everything in Free', 'Priority support'],
			cta: 'Go Pro',
			href: '/auth/signup',
			variant: 'solid'
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

<div class="page-container">
	<Navbar />

	<main class="content">
		<section class="section">
			<div class="container">
				<div class="header">
					<h1>Pricing</h1>
					<p>Every feature is free. You only pay for space.</p>
					<div class="controls">
						<Segmented
							bind:value={cycle}
							options={[
								{ value: 'monthly', label: 'Monthly' },
								{ value: 'annual', label: 'Annual · save 17%' }
							]}
						/>
						<Segmented
							bind:value={currency}
							size="sm"
							options={[
								{ value: 'USD', label: 'USD' },
								{ value: 'EUR', label: 'EUR' },
								{ value: 'INR', label: 'INR' }
							]}
						/>
					</div>
				</div>

				<div class="pricing-grid">
					{#each plans as plan (plan.id)}
						<div class="plan-card" class:highlight={plan.highlight}>
							<div class="plan-top">
								<div class="plan-name">
									<h3>{plan.name}</h3>
									{#if plan.highlight}<Badge tone="accent">Recommended</Badge>{/if}
								</div>
								<div class="price">
									<span class="amount">{plan.amount}</span>
									<span class="period">{plan.per}</span>
								</div>
								<p class="description">{plan.description}</p>
							</div>

							<ul class="features">
								{#each plan.features as feature (feature)}
									<li><Icon icon="ri:check-line" class="check-icon" width="15" /> {feature}</li>
								{/each}
							</ul>

							<Button block variant={plan.variant} href={plan.href}>{plan.cta}</Button>
						</div>
					{/each}
				</div>

				<div class="selfhost">
					<div class="sh-text">
						<span class="sh-title">Self-host</span>
						<span class="sh-desc">Run Silocat on your own hardware. Free forever, AGPL-3.0, no limits.</span>
					</div>
					<Button variant="ghost" href="https://github.com/clickswave/silocat" target="_blank">
						<Icon icon="ri:github-fill" width="16" /> View on GitHub
					</Button>
				</div>

				<p class="footnote">
					Plans are prepaid, no stored card and no auto-renewal. Anonymous drops are always free and
					expire after 7 days; account storage stays until you delete it.
				</p>
			</div>
		</section>
	</main>

	<Footer />
</div>

<style lang="scss">
	.page-container {
		min-height: 100vh;
		display: flex;
		flex-direction: column;
	}
	.content {
		flex: 1;
	}

	.header {
		text-align: center;
		margin-bottom: var(--space-8);
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: var(--space-3);

		h1 {
			font-size: var(--fs-h1);
		}
		p {
			color: var(--ink-mute);
			font-size: var(--fs-body);
			margin-bottom: var(--space-2);
		}
		.controls {
			display: flex;
			gap: var(--space-3);
			flex-wrap: wrap;
			justify-content: center;
		}
	}

	.pricing-grid {
		display: grid;
		grid-template-columns: repeat(3, 1fr);
		gap: var(--space-4);
		align-items: stretch;

		@media (max-width: 860px) {
			grid-template-columns: 1fr;
			max-width: 420px;
			margin-inline: auto;
		}
	}

	.plan-card {
		background: var(--surface);
		border: 1px solid var(--edge);
		border-radius: var(--radius-md);
		padding: var(--space-6);
		display: flex;
		flex-direction: column;
		gap: var(--space-5);

		&.highlight {
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
		justify-content: space-between;
		gap: var(--space-2);
		h3 {
			font-size: var(--fs-body);
			font-weight: var(--fw-semibold);
		}
	}
	.price {
		display: flex;
		align-items: baseline;
		gap: var(--space-1);
		.amount {
			font-size: var(--fs-h2);
			font-weight: var(--fw-semibold);
			letter-spacing: var(--tracking-tight);
		}
		.period {
			color: var(--ink-faint);
			font-size: var(--fs-sm);
		}
	}
	.description {
		color: var(--ink-mute);
		font-size: var(--fs-sm);
		margin: 0;
	}
	.features {
		list-style: none;
		padding: 0;
		margin: 0;
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
		flex: 1;

		li {
			display: flex;
			gap: var(--space-2);
			color: var(--ink-mute);
			font-size: var(--fs-sm);
			line-height: var(--lh-snug);
			:global(.check-icon) {
				color: var(--ink-faint);
				flex-shrink: 0;
				margin-top: 2px;
			}
		}
	}

	.selfhost {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-4);
		flex-wrap: wrap;
		margin-top: var(--space-4);
		padding: var(--space-4) var(--space-5);
		border: 1px solid var(--edge);
		border-radius: var(--radius-md);

		.sh-text {
			display: flex;
			flex-direction: column;
			gap: 2px;
		}
		.sh-title {
			font-size: var(--fs-body);
			font-weight: var(--fw-semibold);
		}
		.sh-desc {
			font-size: var(--fs-sm);
			color: var(--ink-mute);
		}
	}

	.footnote {
		text-align: center;
		margin-top: var(--space-6);
		font-size: var(--fs-sm);
		color: var(--ink-faint);
	}
</style>
