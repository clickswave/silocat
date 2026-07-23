<script>
	import Seo from '$lib/components/Seo.svelte';
	import { breadcrumbSchema } from '$lib/seo.js';
</script>

<Seo
	title="DMCA Policy | SiloCat"
	description="SiloCat's DMCA policy and copyright takedown process, including how to file a valid notice, submit a counter-notice, and our repeat infringer policy."
	schema={breadcrumbSchema([
		{ name: 'Home', path: '/' },
		{ name: 'DMCA Policy', path: '/policies/dmca-policy' }
	])}
/>

<div class="policy-document">
	<h1>DMCA Policy</h1>
	<p class="last-updated">Last Updated: 2026-07-20</p>

	<section>
		<h2>1. Reporting Copyright Infringement</h2>
		<p>
			SiloCat respects intellectual property rights. Even though our <strong>zero-knowledge</strong>
			architecture prevents us from viewing user content, we honour valid copyright takedown notices
			under the Digital Millennium Copyright Act (DMCA) and applicable Indian law.
		</p>
		<p>
			To file a notice, you must provide the <strong>exact URL</strong> of the infringing file. On
			receiving a valid notice, we will delete the corresponding <strong>encrypted blob</strong> from
			our storage backends (Cloudflare R2 or Backblaze B2).
		</p>
	</section>

	<section>
		<h2>2. Elements of a Valid Notice</h2>
		<p>A valid takedown notice must include the following:</p>
		<ul>
			<li>A physical or electronic signature of the copyright owner or an authorised agent.</li>
			<li>Identification of the copyrighted work claimed to have been infringed.</li>
			<li>The <strong>exact URLs</strong> of the material on SiloCat.</li>
			<li>Your contact information, including name, address, telephone number, and email address.</li>
			<li>A statement of good-faith belief that the use is not authorised by the copyright owner, its agent, or the law.</li>
			<li>A statement, under penalty of perjury, that the information in the notice is accurate and that you are authorised to act on behalf of the copyright owner.</li>
		</ul>
	</section>

	<section>
		<h2>3. Submit a Notice</h2>
		<p>Please send copyright takedown notices to our Copyright Agent:</p>
		<p>
			<strong>Copyright Agent</strong>
			<br />
			Clickswave Labs Private Limited
			<br />
			Gujarat, India
			<br />
			Email: team@silo.cat
		</p>
	</section>

	<section>
		<h2>4. Counter-Notice</h2>
		<p>
			If your content was removed and you believe the removal was the result of a mistake or
			misidentification, you may send a counter-notice to team@silo.cat. Your counter-notice must
			include the following:
		</p>
		<ul>
			<li>Your contact information, including name, address, telephone number, and email address.</li>
			<li>Identification of the removed material and the URL at which it appeared before removal.</li>
			<li>A statement, under penalty of perjury, that you have a good-faith belief that the material was removed as a result of mistake or misidentification.</li>
		</ul>
		<p>
			Please note that filing a knowingly false notice or counter-notice may carry legal liability.
		</p>
	</section>

	<section>
		<h2>5. Repeat Infringers</h2>
		<p>
			We terminate the access of users who are repeat infringers of intellectual property rights.
		</p>
	</section>
</div>
