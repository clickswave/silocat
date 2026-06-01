<svelte:head>
	<title>Privacy Policy - SiloCat</title>
	<meta
		name="description"
		content="Review our privacy policy. We operate on a Zero Knowledge basis—your data is yours alone. Kitty powered E2E encrypted anonymous file-sharing."
	/>
	<meta property="og:title" content="Privacy Policy - SiloCat" />
	<meta
		property="og:description"
		content="Kitty powered E2E encrypted anonymous file-sharing and cloud storage platform with parallel downloads."
	/>
</svelte:head>

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

<style>
	.policy-document h1 {
		font-size: 2.5rem;
		margin-bottom: 0.5rem;
	}
	.policy-document .last-updated {
		color: #71717a;
		margin-bottom: 3rem;
	}
	section {
		margin-bottom: 3rem;
	}
	section h2 {
		color: var(--primary, #ff4655);
		font-size: 1.5rem;
		margin-bottom: 1rem;
	}
	section p {
		color: #d4d4d8;
		line-height: 1.7;
		margin-bottom: 1rem;
	}
</style>
