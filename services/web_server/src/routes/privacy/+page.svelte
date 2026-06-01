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
		<div class="hero">
			<h1>Privacy at <span class="text-gradient">SILO.CAT</span></h1>
			<p class="subtitle">
				Your files are encrypted before they ever leave your device - only you hold the keys.
				Without the encryption key, no one can decrypt your files, not even us.
			</p>
			<p class="description">
				We simply don’t have what we cannot give. Lose your keys, and the files are gone forever —
				there’s no recovery, no backdoors, no exceptions.
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
						cannot be opened — not by us, not by anyone.
					</div>
					<div class="col access highlight">Only the Recipient with the Key</div>
				</div>
			</div>
		</div>

		<div class="cta-section">
			<h2>Ready to share securely?</h2>
			<a href="/" class="btn-primary">Start Uploading</a>

			<p class="legal-note">
				For legal details, please consult our <a href="/policies/privacy-policy">Privacy Policy</a>
				and <a href="/policies/terms-of-service">Terms of Service</a>.
			</p>
		</div>
	</main>

	<Footer />

	<div class="bg-effects">
		<div class="glow-spot top"></div>
	</div>
</div>

<style lang="scss">
	:global(body) {
		margin: 0;
		background-color: #0b0b0d;
		color: white;
		font-family: 'Outfit', sans-serif;
	}

	.page-container {
		min-height: 100vh;
		display: flex;
		flex-direction: column;
		position: relative;
		overflow-x: hidden;
	}

	.content {
		padding: 4rem 2rem;
		max-width: 1200px;
		margin: 0 auto;
		text-align: center;
		flex: 1;
		position: relative;
		z-index: 10;
	}

	.hero {
		margin-bottom: 4rem;

		h1 {
			font-size: 3rem;
			font-weight: 800;
			margin-bottom: 1.5rem;
			line-height: 1.2;

			.text-gradient {
				background: linear-gradient(135deg, #fff 0%, #a1a1aa 100%);
				-webkit-background-clip: text;
				-webkit-text-fill-color: transparent;
			}
		}

		.subtitle {
			font-size: 1.25rem;
			color: #e4e4e7;
			max-width: 800px;
			margin: 0 auto 1.5rem;
			line-height: 1.6;
		}

		.description {
			font-size: 1.1rem;
			color: #a1a1aa;
			max-width: 700px;
			margin: 0 auto;
			line-height: 1.6;
		}
	}

	.lifecycle-section {
		margin-bottom: 5rem;
		text-align: left;

		h2 {
			text-align: center;
			font-size: 2rem;
			margin-bottom: 2rem;
			color: var(--primary, #ff4655);
		}

		.lifecycle-table {
			background: rgba(255, 255, 255, 0.03);
			border: 1px solid rgba(255, 255, 255, 0.1);
			border-radius: 16px;
			overflow: hidden;
			display: flex;
			flex-direction: column;

			.table-header {
				display: grid;
				grid-template-columns: 1fr 2fr 1.5fr;
				padding: 1rem 1.5rem;
				background: rgba(255, 255, 255, 0.05);
				border-bottom: 1px solid rgba(255, 255, 255, 0.1);
				font-weight: 700;
				color: white;
				text-transform: uppercase;
				font-size: 0.9rem;
				letter-spacing: 0.05em;

				@media (max-width: 768px) {
					display: none;
				}
			}

			.table-row {
				display: grid;
				grid-template-columns: 1fr 2fr 1.5fr;
				padding: 1.5rem;
				border-bottom: 1px solid rgba(255, 255, 255, 0.05);
				gap: 1rem;
				align-items: center;

				&:last-child {
					border-bottom: none;
				}

				@media (max-width: 768px) {
					display: flex;
					flex-direction: column;
					align-items: flex-start;
					gap: 0.5rem;
				}

				.col {
					color: #d4d4d8;
					line-height: 1.5;
					font-size: 0.95rem;

					&.stage {
						color: var(--primary, #ff4655);
						font-weight: 600;
						font-size: 1rem;
					}

					&.access {
						font-family: monospace;
						font-size: 0.9rem;
						color: #a1a1aa;

						&.highlight {
							color: #4ade80;
						}
					}
				}
			}
		}
	}

	.cta-section {
		h2 {
			font-size: 2rem;
			margin-bottom: 2rem;
		}

		.btn-primary {
			background: var(--primary, #ff4655);
			color: white;
			text-decoration: none;
			padding: 1rem 2.5rem;
			border-radius: 12px;
			font-weight: 600;
			font-size: 1.1rem;
			display: inline-block;
			transition: all 0.2s;

			&:hover {
				background: #e03e4b;
				box-shadow: 0 0 20px rgba(255, 70, 85, 0.4);
			}
		}

		.legal-note {
			margin-top: 2rem;
			color: #52525b;
			font-size: 0.9rem;

			a {
				color: #71717a;
				text-decoration: none;
				&:hover {
					text-decoration: underline;
					color: #a1a1aa;
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
