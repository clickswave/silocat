<script>
	import Navbar from '$lib/components/Navbar.svelte';
	import Footer from '$lib/components/Footer.svelte';
	import Seo from '$lib/components/Seo.svelte';
	import { breadcrumbSchema } from '$lib/seo.js';
	import { Button } from '$lib/ui';

	const stages = [
		{
			stage: 'On your device',
			what: 'Your file is encrypted in your browser with ChaCha20-Poly1305 before upload. You hold the key.',
			access: 'only you',
			ok: true
		},
		{
			stage: 'Upload',
			what: 'Encrypted chunks travel to our servers. Large files are split into chunks, each salted independently.',
			access: 'ciphertext only',
			ok: false
		},
		{
			stage: 'Storage',
			what: 'Encrypted chunks are stored across independent providers. Salts and nonces live separately from the data.',
			access: 'ciphertext only',
			ok: false
		},
		{
			stage: 'Download',
			what: 'The recipient fetches the encrypted chunks. Nothing is readable in transit.',
			access: 'ciphertext only',
			ok: false
		},
		{
			stage: 'Decryption',
			what: 'Decryption happens in the recipient’s browser using the password. Without it, the file cannot be opened, by anyone.',
			access: 'only the recipient',
			ok: true
		}
	];
</script>

<Seo
	title="Privacy & security model | Silocat"
	description="How Silocat protects your data: zero-knowledge architecture and client-side end-to-end encryption mean we can never read your files. Understand the security model."
	schema={breadcrumbSchema([
		{ name: 'Home', path: '/' },
		{ name: 'Privacy', path: '/privacy' }
	])}
/>

<div class="page-container">
	<Navbar />

	<main class="content">
		<section class="section">
			<div class="container narrow">
				<h1>The security model</h1>
				<p class="subtitle">
					Files are encrypted before they leave your device, and only you hold the key. Lose it and
					the file is gone forever. No recovery, no backdoors, no exceptions. We can't hand over
					what we can't read.
				</p>

				<div class="lifecycle">
					{#each stages as row (row.stage)}
						<div class="row">
							<div class="stage">{row.stage}</div>
							<div class="what">{row.what}</div>
							<div class="access" class:ok={row.ok}>{row.access}</div>
						</div>
					{/each}
				</div>

				<p class="verify-note">
					Don't take our word for it. The client-side crypto is
					<a href="https://github.com/clickswave/silocat" target="_blank" rel="noreferrer">open source</a>,
					read it yourself.
				</p>

				<div class="cta-section">
					<Button size="lg" href="/">Start uploading</Button>
					<p class="legal-note">
						For the legal details, see the <a href="/policies/privacy-policy">privacy policy</a>
						and <a href="/policies/terms-of-service">terms of service</a>.
					</p>
				</div>
			</div>
		</section>
	</main>

	<Footer />
</div>

<style lang="scss">
	.page-container {
		min-height: 100vh;
		display: flex;
		flex-direction: column;
	}

	.content {
		flex: 1;
	}

	h1 {
		font-size: var(--fs-h1);
		margin-bottom: var(--space-4);
	}

	.subtitle {
		color: var(--ink-mute);
		line-height: var(--lh-normal);
		margin-bottom: var(--space-8);
		max-width: 60ch;
	}

	.lifecycle {
		display: flex;
		flex-direction: column;
		border-top: 1px solid var(--edge);
	}

	.row {
		display: grid;
		grid-template-columns: 140px 1fr 150px;
		gap: var(--space-4);
		padding: var(--space-4) 0;
		border-bottom: 1px solid var(--edge);
		align-items: baseline;

		@media (max-width: 640px) {
			grid-template-columns: 1fr;
			gap: var(--space-1);
		}
	}

	.stage {
		font-weight: var(--fw-semibold);
		font-size: var(--fs-sm);
	}

	.what {
		color: var(--ink-mute);
		font-size: var(--fs-sm);
		line-height: var(--lh-normal);
	}

	.access {
		font-family: var(--font-mono);
		font-size: var(--fs-xs);
		color: var(--ink-faint);
		text-align: right;

		&.ok {
			color: var(--ok);
		}

		@media (max-width: 640px) {
			text-align: left;
		}
	}

	.verify-note {
		margin-top: var(--space-5);
		font-size: var(--fs-sm);
		color: var(--ink-faint);

		a {
			color: var(--ink-mute);
			text-decoration: underline;
			text-underline-offset: 3px;
			&:hover {
				color: var(--ink);
			}
		}
	}

	.cta-section {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: var(--space-4);
		margin-top: var(--space-10);
	}

	.legal-note {
		color: var(--ink-faint);
		font-size: var(--fs-sm);

		a {
			color: var(--ink-mute);
			&:hover {
				color: var(--ink);
				text-decoration: underline;
			}
		}
	}
</style>
