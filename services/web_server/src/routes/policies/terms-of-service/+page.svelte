<script>
	import Seo from '$lib/components/Seo.svelte';
	import { breadcrumbSchema } from '$lib/seo.js';
</script>

<Seo
	title="Terms of Service | SiloCat"
	description="SiloCat's Terms of Service: the agreement covering our zero-knowledge encrypted storage and sharing service, billing, liability, and your rights."
	schema={breadcrumbSchema([
		{ name: 'Home', path: '/' },
		{ name: 'Terms of Service', path: '/policies/terms-of-service' }
	])}
/>

<div class="policy-document">
	<h1>Terms of Service</h1>
	<p class="last-updated">Last Updated: 2026-07-20</p>

	<section>
		<h2>1. Acceptance of Terms</h2>
		<p>
			By accessing and using SiloCat, including its <strong>Shadow</strong> (anonymous) and
			<strong>Sanctum</strong> (authenticated) services (together, the "Service"), you agree to these
			Terms of Service. The Service is administered by Clickswave Labs Private Limited ("Clickswave",
			"we", "us", or "our"). If you do not agree to these Terms, do not access or use the Service.
		</p>
	</section>

	<section>
		<h2>2. Service Description</h2>
		<p>
			SiloCat provides end-to-end encrypted file storage and sharing. The architecture ensures that
			<strong>all encryption happens client-side</strong>. We (the server) never see your plaintext
			data or your encryption keys. Your encrypted data is managed by our internal storage service,
			<strong>WatchCat</strong>, and stored on distributed backends (Cloudflare R2 / Backblaze B2).
		</p>
		<p>
			You agree to use the Service only for lawful purposes and in compliance with our <a
				href="/policies/acceptable-use">Acceptable Use Policy</a
			>.
		</p>
	</section>

	<section>
		<h2>3. User Responsibilities and Encryption</h2>
		<p>
			You are solely responsible for the content you encrypt and upload. Because SiloCat operates on
			a <strong>zero-knowledge</strong> basis:
		</p>
		<ul>
			<li>
				We <strong>cannot recover your password</strong> or your data if you lose your encryption key
				or share link.
			</li>
			<li>
				We <strong>cannot decrypt</strong> your files to comply with law-enforcement requests (though
				we can delete the encrypted blobs).
			</li>
			<li>
				You retain full ownership of your data, and you grant us only a limited license to store the
				encrypted blobs so that we can provide the Service.
			</li>
		</ul>
	</section>

	<section>
		<h2>4. Subscriptions and Billing</h2>
		<p>
			Silocat offers a free plan (Free) and paid plans (Plus and Pro). Paid plans are
			<strong>prepaid</strong> for the term you choose (monthly or annual) through our payment
			processor, Razorpay. We do not store your card details, and plans
			<strong>do not automatically renew</strong>: you are only ever charged when you actively
			purchase or renew a plan. When a paid term ends without renewal, your account returns to the
			Free plan.
		</p>
		<ul>
			<li>Because plans do not auto-renew, there is no recurring charge and nothing to cancel; a paid plan simply runs until the end of its term.</li>
			<li>
				How refunds and service credits work is set out in our <a href="/policies/refund-policy"
					>Refund Policy</a
				>.
			</li>
			<li>Prices are exclusive of taxes unless stated otherwise, and applicable taxes may be added at the time of billing.</li>
		</ul>
	</section>

	<section>
		<h2>5. Service Availability</h2>
		<p>
			We provide the Service on a reasonable-efforts basis. We do not guarantee that the Service will
			be uninterrupted, error-free, or available at any particular uptime level.
		</p>
		<p>
			If the Service is unusable because of downtime or a defect for which we are responsible, your
			remedy is the service credit described in the <a href="/policies/refund-policy">Refund Policy</a
			>, an extension of your subscription period, and/or an increased storage allowance, as we
			reasonably determine. This is your sole and exclusive remedy for downtime or unavailability.
		</p>
	</section>

	<section>
		<h2>6. Warranties and Liability</h2>
		<p>
			The Service is provided <strong>"as is"</strong> and <strong>"as available"</strong> without
			warranty of any kind, whether express or implied, to the maximum extent permitted by applicable
			law.
		</p>
		<p>
			We are not liable for data loss, whether it arises from you losing your encryption key, a fault
			in our systems, or a failure of a storage backend. Except for liability that cannot be excluded
			or limited under applicable law, our total aggregate liability arising out of or relating to the
			Service or these Terms will not exceed the total fees you paid us in the twelve (12) months
			immediately preceding the event giving rise to the claim.
		</p>
		<p>
			To the maximum extent permitted by law, we will not be liable for any indirect, incidental,
			special, consequential, or punitive loss, including loss of profits, revenue, data, or goodwill.
		</p>
	</section>

	<section>
		<h2>7. Indemnification</h2>
		<p>
			To the maximum extent permitted by applicable law, you agree to defend, indemnify, and hold
			harmless Clickswave Labs Private Limited, together with its directors, officers, employees, and
			agents, from and against any claims, losses, liabilities, damages, costs, and reasonable legal
			fees arising out of or related to: (a) the content you upload to the Service; (b) your use of
			the Service; (c) your breach of these Terms or the <a href="/policies/acceptable-use"
				>Acceptable Use Policy</a
			>; or (d) your violation of any applicable law or the rights of any third party.
		</p>
	</section>

	<section>
		<h2>8. Termination</h2>
		<p>
			We reserve the right to suspend or ban any user or IP address found abusing the Service. If
			WatchCat detects abusive patterns (for example, net abuse or DDoS activity), your access may be
			revoked immediately.
		</p>
		<p>
			Any provision that by its nature should survive termination will survive, including the sections
			on intellectual property and ownership, warranties and liability, indemnification, and governing
			law.
		</p>
	</section>

	<section>
		<h2>9. Changes to These Terms</h2>
		<p>
			We may update these Terms as the Service evolves. Material changes will be flagged, and the
			"Last Updated" date above will be revised. Your continued use of the Service after a change
			takes effect constitutes acceptance of the updated Terms.
		</p>
	</section>

	<section>
		<h2>10. Governing Law</h2>
		<p>
			These Terms are governed by the laws of India. Subject to any right you may have under
			applicable consumer-protection law, the courts located in Gujarat, India will have exclusive
			jurisdiction over any dispute arising out of or relating to these Terms or the Service.
		</p>
	</section>

	<section>
		<h2>11. Contact</h2>
		<p>
			If you have any questions about these Terms, please contact us at <a href="mailto:team@silo.cat"
				>team@silo.cat</a
			>.
		</p>
		<p>
			Operator: Clickswave Labs Private Limited, Gujarat, India.
		</p>
	</section>
</div>
