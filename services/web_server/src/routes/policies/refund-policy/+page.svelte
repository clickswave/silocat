<script>
	import Seo from '$lib/components/Seo.svelte';
	import { breadcrumbSchema } from '$lib/seo.js';
</script>

<Seo
	title="Refund Policy | SiloCat"
	description="SiloCat's refund policy for subscriptions and add-ons."
	schema={breadcrumbSchema([
		{ name: 'Home', path: '/' },
		{ name: 'Refund Policy', path: '/policies/refund-policy' }
	])}
/>

<div class="policy-document">
	<h1>Refund Policy</h1>
	<p class="last-updated">Last Updated: January 4, 2026</p>

	<section>
		<h2>1. Stray (Free) Service</h2>
		<p>
			The **Stray** plan is free. We do not charge for it, so we cannot refund it. Enjoy the free
			bytes.
		</p>
	</section>

	<section>
		<h2>2. Feral (Pro) & Pride (Enterprise)</h2>
		<p>
			**Feral** subscriptions are billed monthly. You may cancel at any time, but we generally do
			not offer refunds for partial billing periods. Access continues until the end of your billing
			cycle.
		</p>
		<p>
			If the Cat ate your money by mistake (billing error), contact support@clickswave.com within 14
			days.
		</p>
	</section>
</div>
