<script>
	import Icon from '@iconify/svelte';
	import Navbar from '$lib/components/Navbar.svelte';
	import Footer from '$lib/components/Footer.svelte';
</script>

<svelte:head>
	<title>Privacy - SiloCat</title>
	<meta
		name="description"
		content="Understand how SiloCat protects your data. Kitty powered E2E encrypted anonymous file-sharing."
	/>
	<meta property="og:title" content="Privacy - SiloCat" />
	<meta
		property="og:description"
		content="Kitty powered E2E encrypted anonymous file-sharing and cloud storage platform with parallel downloads."
	/>
</svelte:head>

<div class="page-container">
	<Navbar />

	<main class="content">
		<section class="section">
			<div class="container">
		<div class="hero">
			<span class="eyebrow">privacy</span>
			<h1>Privacy at <span class="text-gradient">SILO.CAT</span></h1>
			<p class="subtitle">
				Your files are encrypted before they ever leave your device. Only you hold the keys.
				Without the encryption key, no one can decrypt your files, not even us.
			</p>
			<p class="description">
				We simply don't have what we cannot give. Lose your keys, and the files are gone forever.
				There's no recovery, no backdoors, no exceptions.
			</p>
		</div>

		<div class="lifecycle-section">
			<h2>Your File's Privacy Lifecycle</h2>
			<div class="lifecycle-table">
				<div class="table-header">
					<div class="col">Stage</div>
					<div class="col">What Happens</div>
					<div class="col">Who Has Access</div>
				</div>

				<div class="table-row">
					<div class="col stage">On Your Device</div>
					<div class="col">
						Your file is encrypted in your browser using ChaCha20 before upload. You hold the
						encryption key.
					</div>
					<div class="col access highlight">Only You</div>
				</div>

				<div class="table-row">
					<div class="col stage">Upload in Progress</div>
					<div class="col">
						Encrypted file (or chunks) are transmitted to our servers. Large files are split into
						chunks, each salted independently.
					</div>
					<div class="col access">Only Encrypted Data Visible to Us</div>
				</div>

				<div class="table-row">
					<div class="col stage">Cloud Storage</div>
					<div class="col">
						Encrypted files or chunks are stored across multiple silos using Backblaze and
						Cloudflare. Salts and nonces needed for decryption are stored separately.
					</div>
					<div class="col access">We store encrypted data; no access to your original file</div>
				</div>

				<div class="table-row">
					<div class="col stage">Download Process</div>
					<div class="col">
						Your encrypted file or chunks are fetched by the recipient. They must provide the
						correct encryption key to decrypt.
					</div>
					<div class="col access">Only Encrypted Data Transmitted</div>
				</div>

				<div class="table-row">
					<div class="col stage">Decryption</div>
					<div class="col">
						Decryption happens in the recipient’s browser using the key. Without the key, files
						cannot be opened, not by us, not by anyone.
					</div>
					<div class="col access highlight">Only the Recipient with the Key</div>
				</div>
			</div>
		</div>

		<div class="cta-section">
			<h2>Ready to share securely?</h2>
			<a href="/" class="btn btn-primary btn-lg">Start Uploading</a>

			<p class="legal-note">
				For legal details, please consult our <a href="/policies/privacy-policy">Privacy Policy</a>
				and <a href="/policies/terms-of-service">Terms of Service</a>.
			</p>
		</div>
			</div>
		</section>
	</main>

	<Footer />

	<div class="bg-effects">
		<div class="glow-spot top"></div>
	</div>
</div>

<style lang="scss">
	.page-container {
		min-height: 100vh;
		display: flex;
		flex-direction: column;
		position: relative;
		overflow-x: hidden;
	}

	.content {
		text-align: center;
		flex: 1;
		position: relative;
		z-index: 10;
	}

	.hero {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: var(--space-4);
		margin-bottom: var(--space-10);

		h1 {
			font-size: var(--fs-h1);
			font-weight: var(--fw-black);
		}

		.subtitle {
			font-size: var(--fs-lg);
			color: var(--text-primary);
			max-width: 800px;
			margin: 0 auto;
			line-height: var(--lh-normal);
		}

		.description {
			font-size: var(--fs-body);
			color: var(--text-secondary);
			max-width: 700px;
			margin: 0 auto;
			line-height: var(--lh-normal);
		}
	}

	.lifecycle-section {
		margin-bottom: var(--space-12);
		text-align: left;

		h2 {
			text-align: center;
			font-size: var(--fs-h2);
			margin-bottom: var(--space-6);
			color: var(--primary);
		}

		.lifecycle-table {
			background: var(--bg-card);
			border: 1px solid var(--border-default);
			border-radius: var(--radius-md);
			box-shadow: var(--shadow-card);
			overflow: hidden;
			display: flex;
			flex-direction: column;

			.table-header {
				display: grid;
				grid-template-columns: 1fr 2fr 1.5fr;
				padding: var(--space-4) var(--space-5);
				background: var(--tint-soft);
				border-bottom: 1px solid var(--border-default);
				font-weight: var(--fw-bold);
				color: var(--text-primary);
				text-transform: uppercase;
				font-size: var(--fs-sm);
				letter-spacing: 0.05em;

				@media (max-width: 768px) {
					display: none;
				}
			}

			.table-row {
				display: grid;
				grid-template-columns: 1fr 2fr 1.5fr;
				padding: var(--space-5);
				border-bottom: 1px solid var(--hairline);
				gap: var(--space-4);
				align-items: center;

				&:last-child {
					border-bottom: none;
				}

				@media (max-width: 768px) {
					display: flex;
					flex-direction: column;
					align-items: flex-start;
					gap: var(--space-2);
				}

				.col {
					color: var(--text-secondary);
					line-height: var(--lh-snug);
					font-size: var(--fs-sm);

					&.stage {
						color: var(--primary);
						font-weight: var(--fw-semibold);
						font-size: var(--fs-body);
					}

					&.access {
						font-family: var(--font-mono);
						font-size: var(--fs-sm);
						color: var(--text-secondary);

						&.highlight {
							color: var(--success);
						}
					}
				}
			}
		}
	}

	.cta-section {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: var(--space-5);

		h2 {
			font-size: var(--fs-h2);
		}

		.legal-note {
			margin-top: var(--space-2);
			color: var(--text-muted);
			font-size: var(--fs-sm);

			a {
				color: var(--text-secondary);
				text-decoration: none;
				&:hover {
					text-decoration: underline;
					color: var(--text-primary);
				}
			}
		}
	}

	.bg-effects {
		position: absolute;
		inset: 0;
		z-index: 0;
		pointer-events: none;

		.glow-spot {
			position: absolute;
			width: 800px;
			height: 800px;
			background: radial-gradient(circle, rgba(255, 70, 85, 0.08) 0%, transparent 70%);
			filter: blur(100px);

			&.top {
				top: -20%;
				right: -20%;
			}
		}
	}
</style>
