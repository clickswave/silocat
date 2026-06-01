<svelte:head>
	<title>DMCA Policy - SiloCat</title>
</svelte:head>

<div class="policy-document">
	<h1>DMCA Policy</h1>
	<p class="last-updated">Last Updated: January 4, 2026</p>

	<section>
		<h2>1. Reporting Copyright Infringement</h2>
		<p>
			SiloCat respects intellectual property rights. Even though our **Zero Knowledge** architecture
			prevents us from viewing user content, we comply with the Digital Millennium Copyright Act
			(DMCA).
		</p>
		<p>
			To file a takedown notice, you must provide the <strong>exact URL</strong> of the infringing file.
			Upon receiving a valid notice, we will delete the corresponding **encrypted blob** from our storage
			vaults (R2/B2).
		</p>
		<p>Please provide our Copyright Agent with:</p>
		<ul>
			<li>A physical or electronic signature of the copyright owner.</li>
			<li>Description of the copyrighted work.</li>
			<li><strong>Exact URLs</strong> of the material on SiloCat.</li>
			<li>Your contact information.</li>
			<li>A statement of good faith belief that the use is not authorized.</li>
			<li>A statement, under penalty of perjury, that the information is accurate.</li>
		</ul>
	</section>

	<section>
		<h2>2. Submit a Notice</h2>
		<p>
			Please send DMCA notices to:
			<br />
			<strong>Clickswave Legal Dept</strong>
			<br />
			Email: legal@clickswave.com
		</p>
	</section>

	<section>
		<h2>3. Repeat Infringers</h2>
		<p>
			We maintain a policy of terminating the access of users who are repeat infringers of
			intellectual property rights.
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
