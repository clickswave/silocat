<script>
	import Seo from '$lib/components/Seo.svelte';
	import { breadcrumbSchema } from '$lib/seo.js';
</script>

<Seo
	title="Disclaimer | SiloCat"
	description="Legal disclaimer for SiloCat, the zero-knowledge, end-to-end encrypted file sharing and cloud storage service operated by Clickswave Labs Private Limited."
	schema={breadcrumbSchema([
		{ name: 'Home', path: '/' },
		{ name: 'Disclaimer', path: '/policies/disclaimer' }
	])}
/>

<div class="policy-document">
	<h1>Disclaimer</h1>
	<p class="last-updated">Last Updated: 2026-07-20</p>

	<section>
		<h2>1. "As Is" and Encryption Risks</h2>
		<p>
			SiloCat is a zero-knowledge, end-to-end encrypted file sharing and cloud storage service.
			The Service is provided on an "as is" and "as available" basis, without guarantees of any
			kind regarding its operation or availability.
		</p>
		<p>
			<strong>Critical warning:</strong> because encryption happens on your device (client-side),
			we never hold your keys and cannot see your files.
			<strong>
				If you lose your URL, key, or password, your data is permanently unrecoverable.
			</strong>
			We cannot recover it, and we cannot reset it. This is a deliberate consequence of the
			privacy design, and you accept this risk when you use the Service.
		</p>
	</section>

	<section>
		<h2>2. No Warranty</h2>
		<p>
			To the maximum extent permitted by applicable law, we disclaim all warranties of any kind,
			whether express or implied, including without limitation the implied warranties of
			merchantability, fitness for a particular purpose, and non-infringement.
		</p>
		<p>
			We do not warrant that the Service will be uninterrupted, secure, timely, or error-free, that
			any defects will be corrected, or that the Service or the systems that make it available are
			free of harmful components.
		</p>
	</section>

	<section>
		<h2>3. Limitation of Liability</h2>
		<p>
			We are not liable for data loss, whether it is caused by you losing your key, password, or
			URL, by a fault in our systems (including the WatchCat maintenance service), or by a failure
			of a storage backend such as Cloudflare R2 or Backblaze B2.
		</p>
		<p>
			<strong>Always keep your own local backups of critical files.</strong> The Service is not a
			substitute for an independent backup of anything you cannot afford to lose.
		</p>
		<p>
			Our liability in connection with the Service is limited as set out in our
			<a href="/policies/terms-of-service">Terms of Service</a>.
		</p>
	</section>

	<p>
		Operator: Clickswave Labs Private Limited, Gujarat, India. Contact: team@silo.cat.
	</p>
</div>
