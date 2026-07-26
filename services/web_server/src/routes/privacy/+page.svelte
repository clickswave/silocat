<script>
	import Seo from '$lib/components/Seo.svelte';
	import { breadcrumbSchema } from '$lib/seo.js';
	import Navbar from '$lib/components/Navbar.svelte';
	import Footer from '$lib/components/Footer.svelte';

	// Access tone: `ok` for "nobody but you", neutral for anything wider. The
	// table is the page, so every row has to be literally true of this build.
	const stages = [
		{
			stage: 'Select',
			what: 'You pick files in your browser. Nothing has moved yet.',
			who: 'Only you',
			tone: 'ok'
		},
		{
			stage: 'Encrypt',
			what: 'libsodium encrypts each chunk locally, with a key Argon2 derives from your password on your device.',
			who: 'Only you',
			tone: 'ok'
		},
		{
			stage: 'Upload',
			what: 'Only ciphertext travels. TLS wraps it a second time in transit.',
			who: 'Only you',
			tone: 'ok'
		},
		{
			stage: 'Store',
			what: 'Our servers hold encrypted blobs plus minimal metadata: size, timestamps, a content hash.',
			who: 'You + metadata: us',
			tone: 'mute'
		},
		{
			stage: 'Share',
			// Deliberately not the handoff's "the key rides in the link fragment":
			// this build shares a password out of band and the link alone is
			// useless without it. Describing the wrong mechanism would change how
			// people handle their passwords, so the row states what actually ships.
			what: 'The link carries no key. You pass the password separately, so a leaked link on its own decrypts nothing.',
			who: 'Anyone with link + password',
			tone: 'mute'
		},
		{
			stage: 'Download',
			what: "The recipient's browser fetches ciphertext and decrypts it locally, once they enter the password.",
			who: 'Only you',
			tone: 'ok'
		},
		{
			stage: 'Delete',
			what: 'Blobs are unlinked immediately and scrubbed from storage in the next sweep.',
			who: 'No one',
			tone: 'ok'
		}
	];
</script>

<Seo
	title="Privacy & security model | Silocat"
	description="How Silocat protects your data: zero-knowledge architecture and client-side end-to-end encryption mean we can never read your files. Understand the security model."
	schema={breadcrumbSchema([
		{ name: 'Home', path: '/' },
		{ name: 'Security', path: '/privacy' }
	])}
/>

<div class="page">
	<Navbar />

	<main class="main">
		<section class="head">
			<h1>The security model</h1>
			<p class="sub">
				Zero knowledge means exactly that: your files are encrypted before they leave your device,
				and the key never touches our servers. We could not read your files if we wanted to, were
				paid to, or were ordered to.
			</p>
		</section>

		<section class="table-section">
			<div class="table">
				<div class="thead">
					<span>Stage</span>
					<span>What happens</span>
					<span>Who can access it</span>
				</div>
				{#each stages as s (s.stage)}
					<div class="row">
						<span class="stage">{s.stage}</span>
						<span class="what">{s.what}</span>
						<span class="who {s.tone}">{s.who}</span>
					</div>
				{/each}
			</div>

			<p class="verify">
				Don't take our word for it. The whole thing is open source:
				<a href="https://github.com/clickswave/silocat" target="_blank" rel="noreferrer">
					read the encryption code yourself
				</a>.
			</p>
		</section>

		<section class="cta">
			<h2>Send something no one else can read.</h2>
			<a href="/" class="cta-btn">Try it now, no account</a>
			<span class="legal-note">
				Looking for the formal policies? They live in the
				<a href="/policies/privacy-policy">Legal Center</a>.
			</span>
		</section>
	</main>

	<Footer />
</div>

<style lang="scss">
	.page {
		min-height: 100vh;
		display: flex;
		flex-direction: column;
		background: var(--bg);
		color: var(--ink);
		font-family: var(--font-sans);
		font-size: var(--fs-body);
		line-height: var(--lh-normal);
	}

	.main {
		flex: 1;
		width: 100%;
		max-width: var(--container);
		margin: 0 auto;
		padding-inline: var(--gutter);
	}

	.head {
		max-width: var(--container-narrow);
		margin: 0 auto;
		padding: clamp(2.5rem, 7vw, 4.5rem) 0 var(--space-6);
		display: flex;
		flex-direction: column;
		gap: var(--space-4);

		h1 {
			margin: 0;
			font-size: var(--fs-h1);
			font-weight: var(--fw-black);
			letter-spacing: var(--tracking-tight);
			line-height: var(--lh-tight);
		}
	}

	.sub {
		margin: 0;
		font-size: var(--fs-lg);
		color: var(--ink-mute);
	}

	.table-section {
		max-width: 860px;
		margin: 0 auto;
		padding-bottom: var(--space-6);
	}

	.table {
		border: 1px solid var(--edge);
		border-radius: var(--radius-md);
		background: var(--surface);
		overflow: hidden;
	}

	.thead,
	.row {
		display: grid;
		grid-template-columns: 0.7fr 1.6fr 0.8fr;
		gap: var(--space-4);
	}

	.thead {
		padding: 0.625rem 1.25rem;
		border-bottom: 1px solid var(--edge);
		font-size: var(--fs-xs);
		color: var(--ink-faint);
	}

	.row {
		padding: 0.875rem 1.25rem;
		border-bottom: 1px solid var(--edge);

		&:last-child {
			border-bottom: 0;
		}
	}

	.stage {
		font-family: var(--font-mono);
		font-size: var(--fs-sm);
		font-weight: var(--fw-medium);
	}

	.what {
		font-size: var(--fs-sm);
		color: var(--ink-mute);
	}

	.who {
		font-size: var(--fs-sm);
		font-weight: var(--fw-medium);

		&.ok {
			color: var(--ok);
		}
		&.mute {
			color: var(--ink-mute);
		}
	}

	.verify {
		margin: 0;
		padding: var(--space-4) 0.125rem 0;
		font-size: var(--fs-sm);
		color: var(--ink-mute);
	}

	.cta {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: var(--space-4);
		padding: clamp(2rem, 6vw, 4rem) 0 clamp(3rem, 8vw, 5rem);
		text-align: center;

		h2 {
			margin: 0;
			font-size: var(--fs-h2);
			font-weight: var(--fw-semibold);
			letter-spacing: var(--tracking-tight);
		}
	}

	.cta-btn {
		display: flex;
		align-items: center;
		height: 46px;
		padding-inline: 1.375rem;
		border-radius: var(--radius-md);
		background: var(--accent);
		color: #fff;
		font-size: 1rem;
		font-weight: var(--fw-medium);
		text-decoration: none;
		transition: background var(--dur-fast) var(--ease);

		&:hover {
			background: var(--accent-hover);
			color: #fff;
		}
	}

	.legal-note {
		font-size: var(--fs-xs);
		color: var(--ink-faint);
	}

	@media (max-width: 720px) {
		.thead {
			display: none;
		}
		.row {
			grid-template-columns: 1fr;
			gap: var(--space-1);
		}
	}
</style>
