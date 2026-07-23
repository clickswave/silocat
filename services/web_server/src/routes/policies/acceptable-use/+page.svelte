<script>
	import Seo from '$lib/components/Seo.svelte';
	import { breadcrumbSchema } from '$lib/seo.js';
</script>

<Seo
	title="Acceptable Use Policy | SiloCat"
	description="SiloCat's Acceptable Use Policy: the content and activities that are prohibited on our zero-knowledge platform, your responsibilities, and how we enforce these rules."
	schema={breadcrumbSchema([
		{ name: 'Home', path: '/' },
		{ name: 'Acceptable Use Policy', path: '/policies/acceptable-use' }
	])}
/>

<div class="policy-document">
	<h1>Acceptable Use Policy</h1>
	<p class="last-updated">Last Updated: 2026-07-20</p>

	<p>
		SiloCat is a zero-knowledge, end-to-end encrypted file sharing and cloud storage service built
		for everyone, including security researchers. Because we cannot read your encrypted content, we
		rely on this policy to keep the platform safe and lawful. By using the Service, you agree to the
		terms below.
	</p>

	<section>
		<h2>1. Prohibited Content</h2>
		<p>You agree not to upload:</p>
		<ul>
			<li>
				<strong>Illegal Content:</strong> CSAM, non-consensual sexual content, or any content that is
				illegal under applicable Indian or international law.
			</li>
			<li>
				<strong>Terrorist Material:</strong> Content promoting terrorism or violent extremism.
			</li>
			<li>
				<strong>Malicious Distribution:</strong> Active malware intended for infection or botnet
				distribution. Inert, password-protected research samples are permitted.
			</li>
		</ul>
	</section>

	<section>
		<h2>2. Prohibited Activities</h2>
		<p>You may not:</p>
		<ul>
			<li>
				Abuse the WatchCat infrastructure (for example, attempting to overload storage backends or
				bypass quotas).
			</li>
			<li>Reverse engineer our systems or internal APIs.</li>
			<li>Use SiloCat as a CDN for hotlinking high-traffic assets.</li>
			<li>Send spam or unsolicited advertising.</li>
			<li>
				Attempt to break, probe, or circumvent the platform's security or access controls without
				authorization.
			</li>
		</ul>
	</section>

	<section>
		<h2>3. Your Responsibility</h2>
		<p>
			You are solely responsible for the content you upload and for the legality of your use of the
			Service. To the extent permitted by law, you agree to indemnify and hold harmless Clickswave
			Labs Private Limited against any claim, loss, or penalty arising from your use of the Service
			in breach of this policy or the law.
		</p>
	</section>

	<section>
		<h2>4. Enforcement</h2>
		<p>
			We reserve the right to remove content by deleting the encrypted blob and to ban users or IP
			addresses who violate this policy. We cooperate with valid legal process, and we report
			serious or unlawful violations, such as CSAM or terrorism, to the relevant authorities.
		</p>
	</section>

	<p>Operator: Clickswave Labs Private Limited, Gujarat, India. Contact: team@silo.cat.</p>
</div>
