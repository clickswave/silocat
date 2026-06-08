<script>
	import Seo from '$lib/components/Seo.svelte';
	import { breadcrumbSchema } from '$lib/seo.js';
</script>

<Seo
	title="Privacy Policy | SiloCat"
	description="SiloCat's privacy policy. We operate on a zero-knowledge basis: your files are end-to-end encrypted and your data is yours alone."
	schema={breadcrumbSchema([
		{ name: 'Home', path: '/' },
		{ name: 'Privacy Policy', path: '/policies/privacy-policy' }
	])}
/>

<div class="policy-document">
	<h1>Privacy Policy</h1>
	<p class="last-updated">Last Updated: January 4, 2026</p>

	<section>
		<h2>1. Zero Knowledge Philosophy</h2>
		<p>
			SiloCat is built on a simple premise: <strong>The server knows nothing.</strong>
			We collect the absolute minimum metadata required to facilitate the transfer. Your IP address is
			used for abuse prevention (rate limiting) but is not permanently linked to your files in a readable
			format.
		</p>
	</section>

	<section>
		<h2>2. Encryption & Storage Architecture</h2>
		<p>
			<strong>Client-Side:</strong> Files are encrypted in your browser using
			<strong>ChaCha20-Poly1305</strong>. The keys are never sent to our servers.
		</p>
		<p>
			<strong>Storage (WatchCat):</strong> Our internal service, WatchCat, places your encrypted chunks
			into secure vaults (Cloudflare R2 or Backblaze B2). To us, your data is just random noise/blobs.
			We cannot read it, mine it, or sell it.
		</p>
	</section>

	<section>
		<h2>3. Metadata & Logs</h2>
		<p>
			We store minimal encrypted metadata in our PostgreSQL database (e.g., file size, mime type,
			expiry). For **Shadow** (anonymous) uploads, this data evaporates after 7 days. For
			**Sanctum** (authenticated) uploads, it persists until you delete it. We do not use
			third-party tracking pixels or analytics.
		</p>
	</section>

	<section>
		<h2>4. Sharing & Access</h2>
		<p>
			Since the decryption key is part of the URL fragment (the part after the '#'), <strong
				>only people with the full link</strong
			> can decrypt the file. If you share the link, you share the key. We don't have it.
		</p>
	</section>

	<section>
		<h2>5. Contact Us</h2>
		<p>
			For privacy concerns or to check if WatchCat is purring correctly: privacy@clickswave.com.
		</p>
	</section>
</div>
