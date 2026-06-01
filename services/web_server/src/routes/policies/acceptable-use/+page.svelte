<svelte:head>
	<title>Acceptable Use Policy - SiloCat</title>
</svelte:head>

<div class="policy-document">
	<h1>Acceptable Use Policy</h1>
	<p class="last-updated">Last Updated: January 4, 2026</p>

	<section>
		<h2>1. Prohibited Content</h2>
		<p>
			SiloCat is designed for everyone, including security researchers. However, lines must be
			drawn. You agree not to upload:
		</p>
		<ul>
			<li>
				<strong>Illegal Content:</strong> CSAM, non-consensual sexual content, or content illegal under
				applicable US/International laws.
			</li>
			<li>
				<strong>Terrorist Material:</strong> Content promoting terrorism or violent extremism.
			</li>
			<li>
				<strong>Malicious Distribution:</strong> Active malware intended for infection/botnet distribution.
				(Research samples/dumps are permitted if password-protected/inert).
			</li>
		</ul>
	</section>

	<section>
		<h2>2. Prohibited Activities</h2>
		<p>You may not:</p>
		<ul>
			<li>
				Abuse the **WatchCat** infrastructure (e.g., attempt to overload storage backends or bypass
				quotas).
			</li>
			<li>Reverse engineer the `api_switch` or internal APIs.</li>
			<li>Use SiloCat as a CDN for hotlinking high-traffic assets (we are a silo, not a hose).</li>
			<li>Spam or unsolicited advertising.</li>
		</ul>
	</section>

	<section>
		<h2>3. Enforcement</h2>
		<p>
			The Cat Bites Back. We reserve the right to remove content (delete the encrypted blob) and ban
			users who violate this policy. We cooperate with law enforcement for serious violations (CSAM,
			terrorism).
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
	ul {
		color: #d4d4d8;
		line-height: 1.7;
		margin-bottom: 1rem;
		padding-left: 1.5rem;
	}
	li {
		margin-bottom: 0.5rem;
	}
</style>
