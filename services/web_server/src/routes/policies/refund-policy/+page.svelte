<script>
	import Seo from '$lib/components/Seo.svelte';
	import { breadcrumbSchema } from '$lib/seo.js';
</script>

<Seo
	title="Refund Policy | SiloCat"
	description="SiloCat's refund policy: how cancellation works, how we compensate for downtime with service credits, and when a payment is refundable."
	schema={breadcrumbSchema([
		{ name: 'Home', path: '/' },
		{ name: 'Refund Policy', path: '/policies/refund-policy' }
	])}
/>

<div class="policy-document">
	<h1>Refund Policy</h1>
	<p class="last-updated">Last Updated: 2026-07-20</p>

	<section>
		<h2>1. Who This Applies To</h2>
		<p>
			This policy covers the paid Silocat plans (<strong>Plus</strong> and <strong>Pro</strong>)
			purchased from <strong>Clickswave Labs Private Limited</strong>, Gujarat, India. Payments are
			processed by Razorpay. It forms part of our
			<a href="/policies/terms-of-service">Terms of Service</a>. The free
			<strong>Free</strong> plan carries no charge, so there is nothing to refund.
		</p>
	</section>

	<section>
		<h2>2. Downtime: We Make Your Storage Whole</h2>
		<p>
			SiloCat is a usage-based service, so when the fault is on our side our first remedy is to
			restore the value you paid for rather than to unwind the payment. If the Service is unavailable
			or unusable because of downtime or a defect we are responsible for, we will put it right by
			<strong>extending your subscription for the affected time and/or increasing your storage
			allowance</strong> by a fair amount.
		</p>
		<p>
			We assess the remedy in good faith from our own monitoring, based on how long the disruption
			lasted and how much it affected you. A service credit of this kind is the primary way we
			compensate for downtime. To help us act quickly, report a problem to team@silo.cat within 14
			days of it happening.
		</p>
	</section>

	<section>
		<h2>3. Cancellation</h2>
		<p>
			Paid plans are prepaid and do not auto-renew, so there is nothing to cancel and no recurring
			charge to stop. Your paid plan simply runs until the end of the term you purchased, after which
			your account returns to the Free plan. We do not provide partial or pro-rated refunds for the
			unused part of a term.
		</p>
	</section>

	<section>
		<h2>4. When a Payment Is Refundable</h2>
		<p>
			Paid plans are billed in advance and are, as a rule, non-refundable once a billing period has
			begun, because you have full access to the plan for that period. We make these exceptions:
		</p>
		<ul>
			<li>
				First payment of a new subscription: if SiloCat is not right for you, you may request a
				refund of that first payment within 7 days, provided the account has not made material use
				of the storage or transfer allowance.
			</li>
			<li>
				Billing errors: a duplicate charge, an amount that does not match the plan you chose, or a
				charge taken after a valid cancellation is refunded in full whenever you report it.
			</li>
			<li>
				Rights you cannot waive: nothing in this policy removes any refund or cancellation right you
				have that cannot be waived under the consumer law that applies to you.
			</li>
		</ul>
		<p>
			Renewal charges, and payments outside these cases, are non-refundable. Where we do approve a
			refund, we may instead offer an equivalent service credit if you prefer it.
		</p>
	</section>

	<section>
		<h2>5. How to Reach Us</h2>
		<p>
			Email team@silo.cat from the address on your account, with the payment date and a short
			description of the issue. We aim to respond within 3 business days. Where we agree a service
			credit, we apply it to your account directly. Where we agree a cash refund, it is issued to
			your original payment method through Razorpay and typically reaches you within 5 to 10 business
			days, depending on your bank or card provider. If a charge looks wrong, please contact us first
			rather than raising a chargeback, so we can fix it quickly.
		</p>
	</section>
</div>
