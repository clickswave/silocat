<script>
	import Seo from '$lib/components/Seo.svelte';
	import { breadcrumbSchema } from '$lib/seo.js';
</script>

<Seo
	title="Privacy Policy | SiloCat"
	description="SiloCat's privacy policy. We operate on a zero-knowledge basis: your files are end-to-end encrypted, and we comply with India's DPDP Act, 2023."
	schema={breadcrumbSchema([
		{ name: 'Home', path: '/' },
		{ name: 'Privacy Policy', path: '/policies/privacy-policy' }
	])}
/>

<div class="policy-document">
	<h1>Privacy Policy</h1>
	<p class="last-updated">Last Updated: 2026-07-20</p>

	<section>
		<h2>1. Who We Are</h2>
		<p>
			SiloCat is operated by <strong>Clickswave Labs Private Limited</strong>, a company incorporated in
			Gujarat, India. For the personal data described in this policy, Clickswave Labs Private Limited
			acts as the <strong>Data Fiduciary</strong> under India's Digital Personal Data Protection Act,
			2023 (the "DPDP Act"). We are committed to protecting your privacy and to processing personal data
			lawfully and transparently.
		</p>
		<p>
			If you have any questions about this policy or about how we handle your data, you can reach our
			Grievance Officer at team@silo.cat.
		</p>
	</section>

	<section>
		<h2>2. Zero Knowledge Philosophy</h2>
		<p>
			SiloCat is built on a simple premise: <strong>The server knows nothing.</strong>
			We collect the absolute minimum metadata required to facilitate the transfer. Your IP address is
			used for abuse prevention and rate limiting, but it is not permanently linked to your files in a
			readable form.
		</p>
	</section>

	<section>
		<h2>3. Encryption and Storage Architecture</h2>
		<p>
			<strong>Client-Side:</strong> Files are encrypted in your browser using
			<strong>ChaCha20-Poly1305</strong>. The keys are never sent to our servers.
		</p>
		<p>
			<strong>Storage (WatchCat):</strong> Our internal service, WatchCat, places your encrypted blobs
			into secure storage (Cloudflare R2 or Backblaze B2). To us, your data is nothing more than
			opaque encrypted blobs. We cannot read it, mine it, or sell it.
		</p>
	</section>

	<section>
		<h2>4. Metadata and Logs</h2>
		<p>
			We store minimal metadata in our database (for example, file size, mime type, and expiry). For
			<strong>Shadow</strong> (anonymous) uploads, this data evaporates after 7 days. For
			<strong>Sanctum</strong> (authenticated) uploads, it persists until you delete it. We do not use
			third-party tracking pixels or analytics.
		</p>
	</section>

	<section>
		<h2>5. Sharing and Access</h2>
		<p>
			Since the decryption key is part of the URL fragment (the part after the '#'), <strong
				>only people with the full link</strong
			> can decrypt the file. If you share the link, you share the key, and we do not have it.
		</p>
	</section>

	<section>
		<h2>6. Who We Share Data With</h2>
		<p>
			We do not sell your data. We rely on a small number of trusted processors, and only to the
			extent needed to run the service:
		</p>
		<ul>
			<li><strong>Razorpay</strong> for payment processing.</li>
			<li><strong>Cloudflare and Backblaze</strong> for encrypted storage.</li>
			<li>Our transactional email provider for account and service notifications.</li>
		</ul>
		<p>
			Some of these processors operate infrastructure outside India, so your data may be processed in
			other countries under the transfer conditions permitted by the DPDP Act. We may also disclose
			data where we are required to do so by a valid legal order.
		</p>
	</section>

	<section>
		<h2>7. Your Rights</h2>
		<p>
			Under the DPDP Act, you have the right to access a summary of your personal data, to correct or
			complete it, to request its erasure, to withdraw consent, and to raise a grievance. To exercise
			these rights, email team@silo.cat from your account address so we can verify your request. Please
			note that some data may be retained where the law requires us to keep it.
		</p>
	</section>

	<section>
		<h2>8. Children</h2>
		<p>
			SiloCat is not intended for anyone under 18 years of age. We do not knowingly collect personal
			data from minors. If you believe a minor has provided us with personal data, please contact us so
			we can address it.
		</p>
	</section>

	<section>
		<h2>9. Grievance Officer</h2>
		<p>
			If you have concerns about how your personal data is handled, you may contact our Grievance
			Officer:
		</p>
		<ul>
			<li><strong>Grievance Officer:</strong> Dikshant Chandel</li>
			<li><strong>Entity:</strong> Clickswave Labs Private Limited</li>
			<li><strong>Location:</strong> Gujarat, India</li>
			<li><strong>Email:</strong> team@silo.cat</li>
		</ul>
	</section>

	<section>
		<h2>10. Changes</h2>
		<p>
			We may update this policy from time to time. The Last Updated date above reflects the most
			recent changes, and where a change significantly affects your rights, we will notify you. We
			appreciate you trusting SiloCat to keep your data private and your WatchCat purring.
		</p>
	</section>
</div>
