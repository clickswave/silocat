<svelte:head>
	<title>Terms of Service - SiloCat</title>
	<meta
		name="description"
		content="Read our Terms of Service. Fair use, abuse prevention, and your rights on SiloCat. Kitty powered E2E encrypted anonymous file-sharing."
	/>
	<meta property="og:title" content="Terms of Service - SiloCat" />
	<meta
		property="og:description"
		content="Kitty powered E2E encrypted anonymous file-sharing and cloud storage platform with parallel downloads."
	/>
</svelte:head>

<div class="policy-document">
	<h1>Terms of Service</h1>
	<p class="last-updated">Last Updated: January 4, 2026</p>

	<section>
		<h2>1. Acceptance of Terms</h2>
		<p>
			By accessing and using SiloCat, including its <strong>Shadow</strong> (anonymous) and
			<strong>Sanctum</strong> (authenticated) services, you agree to these Terms of Service. The Service
			is administered by Clickswave ("Clickswave Labs Private Limited"). If you do not agree, do not prowl
			here.
		</p>
	</section>

	<section>
		<h2>2. Service Description</h2>
		<p>
			SiloCat provides end-to-end encrypted file storage and sharing. The architecture ensures that
			**all encryption happens client-side**. We (the server) never see your plaintext data or your
			encryption keys. Your encrypted data is managed by our internal sentinel, **WatchCat**, and
			stored on distributed backends (Cloudflare R2 / Backblaze B2).
		</p>
		<p>
			You agree to use the Service only for lawful purposes in compliance with our <a
				href="/policies/acceptable-use">Acceptable Use Policy</a
			>.
		</p>
	</section>

	<section>
		<h2>3. User Responsibilities & Encryption</h2>
		<p>
			You are solely responsible for the content you encryption and upload. Because SiloCat operates
			on a <strong>Zero Knowledge</strong> basis:
		</p>
		<ul>
			<li>We <strong>cannot recover your password</strong> or data if you lose your key/link.</li>
			<li>
				We **cannot decrypt** your files to comply with law enforcement requests (though we can
				delete the encrypted blobs).
			</li>
			<li>
				You retain full ownership of your data, granting us only the license to store the encrypted
				blobs.
			</li>
		</ul>
	</section>

	<section>
		<h2>4. Termination</h2>
		<p>
			We reserve the right to ban any user or IP address found abusing the service. If WatchCat
			detects abusive patterns (e.g., net abuse, DDoS), your access may be revoked immediately.
		</p>
	</section>

	<section>
		<h2>5. Changes to Terms</h2>
		<p>
			We may update these terms as the platform evolves. Significant changes will be flagged.
			Continued use of the service constitutes acceptance.
		</p>
	</section>
</div>
