<script>
	import Navbar from '$lib/components/Navbar.svelte';
	import Footer from '$lib/components/Footer.svelte';
	import Icon from '$lib/ui/Icon.svelte';
	import Seo from '$lib/components/Seo.svelte';
	import { breadcrumbSchema } from '$lib/seo.js';
	import { slide } from 'svelte/transition';
	import { toast } from '$lib/toast.js';

	const QUICK_START = `import { Silocat } from '@clickswave/silocat-client';

const silo = new Silocat({ apiKey: process.env.SILOCAT_API_KEY });

// Encrypted in your process. Ciphertext is all that leaves it.
const file = await silo.upload(bytes, {
  name: 'contract.pdf',
  password: 'correct-horse-battery-staple'
});

// A link anyone can open. It carries no key.
const { url } = await silo.share(file.id, { type: 'public' });

// Read it back.
const plaintext = await silo.download(file.id, {
  password: 'correct-horse-battery-staple'
});`;

	const METHODS = [
		{ sig: 'storage()', desc: 'Bytes used, total and free.' },
		{ sig: 'listFiles({ folderId, starred, shared })', desc: 'Omit folderId for the root.' },
		{ sig: 'listFolders({ parentId })', desc: 'Folders at one level.' },
		{ sig: 'createFolder(name, { parentId })', desc: 'Returns the new folder.' },
		{
			sig: 'upload(data, { name, password, mime, folderId, onProgress })',
			desc: 'Uint8Array, ArrayBuffer or Blob. Omit password to upload unencrypted, readable by us.'
		},
		{ sig: 'download(id, { password, onProgress })', desc: 'Returns plaintext bytes.' },
		{ sig: "share(id, { type })", desc: "'public', 'once' or 'off'." },
		{ sig: 'trash(id) / restore(id)', desc: 'Recoverable until the retention window expires.' },
		{ sig: 'deleteForever(id)', desc: 'Irreversible.' }
	];

	// Only endpoints an API key can actually call. Everything the browser-only
	// or admin surfaces expose is deliberately absent: documenting a route
	// nobody outside the app can reach is worse than not documenting it.
	const GROUPS = [
		{
			title: 'Account',
			desc: 'Who you are and how much room you have.',
			endpoints: [
				{
					method: 'GET',
					path: '/api/v1/sanctum/user/storage',
					desc: 'Storage used, total and free, in bytes.',
					sample: { success: { used: 2206833100, total: 10737418240, free: 8530585140 } }
				}
			]
		},
		{
			title: 'Browsing',
			desc: 'Listing what you already have.',
			endpoints: [
				{
					method: 'GET',
					path: '/api/v1/sanctum/file/list',
					desc: 'Files, optionally filtered by folder_id, starred or shared.',
					sample: {
						data: {
							files: [
								{
									id: 'f_01H…',
									name: 'contract.pdf',
									size: 1887436,
									mime: 'application/pdf',
									encrypted: true,
									starred: false,
									share_type: 'off'
								}
							]
						}
					}
				},
				{
					method: 'POST',
					path: '/api/v1/sanctum/folder/list',
					desc: 'Folders under parent_id, or the root when null.',
					body: { parent_id: null }
				},
				{
					method: 'POST',
					path: '/api/v1/sanctum/folder/create',
					desc: 'Create a folder.',
					body: { name: 'Contracts', parent_id: null }
				}
			]
		},
		{
			title: 'Upload',
			desc: 'Three calls per file. The library does all of this for you.',
			endpoints: [
				{
					method: 'POST',
					path: '/api/v1/sanctum/file',
					desc: 'Register a file and receive one presigned PUT per chunk.',
					body: {
						storage_type: 'sanctum',
						file_encrypted: true,
						file_name: 'contract.pdf',
						file_mime: 'application/pdf',
						file_size: 1887436,
						sha256_checksum: 'e3b0c442…',
						public_access: false,
						folder_id: null,
						chunks: [
							{ start: 0, end: 1887436, size: 1887436, checksum: 'pending', salt: 'base64…', nonce: 'base64…' }
						]
					}
				},
				{
					method: 'PUT',
					path: '{presigned_url}',
					desc: 'PUT the ciphertext for one chunk directly to storage.',
					sample: '(request body is raw ciphertext; the response is empty)'
				},
				{
					method: 'POST',
					path: '/api/v1/sanctum/file/mark-chunk-complete',
					desc: 'Confirm a chunk landed. Unconfirmed uploads are reaped.',
					body: { chunk_id: 'c_01H…' }
				}
			]
		},
		{
			title: 'Download',
			desc: 'Fetch ciphertext, then decrypt locally.',
			endpoints: [
				{
					method: 'POST',
					path: '/api/v1/sanctum/file/fetch-chunks',
					desc: 'Presigned GETs plus the salt and per-chunk nonces you need to decrypt.',
					body: { file_id: 'f_01H…' }
				}
			]
		},
		{
			title: 'Sharing',
			desc: 'Links never carry the decryption password.',
			endpoints: [
				{
					method: 'POST',
					path: '/api/v1/sanctum/file/share/toggle',
					desc: "Set share_type to 'public', 'once' or 'off'.",
					body: { file_id: 'f_01H…', share_type: 'public' }
				},
				{
					method: 'POST',
					path: '/api/v1/sanctum/file/share/regenerate',
					desc: 'Issue a new token. The old link stops working immediately.',
					body: { file_id: 'f_01H…' }
				}
			]
		},
		{
			title: 'Lifecycle',
			desc: 'Trash is recoverable. Permanent delete is not.',
			endpoints: [
				{ method: 'POST', path: '/api/v1/sanctum/file/delete', desc: 'Move to trash.', body: { file_id: 'f_01H…' } },
				{ method: 'POST', path: '/api/v1/sanctum/file/restore', desc: 'Restore from trash.', body: { file_id: 'f_01H…' } },
				{
					method: 'POST',
					path: '/api/v1/sanctum/file/permanent-delete',
					desc: 'Delete forever. Ciphertext is scrubbed from storage.',
					body: { file_id: 'f_01H…' }
				},
				{ method: 'POST', path: '/api/v1/sanctum/file/star', desc: 'Star or unstar.', body: { file_id: 'f_01H…', starred: true } },
				{ method: 'POST', path: '/api/v1/sanctum/file/update', desc: 'Rename or move.', body: { file_id: 'f_01H…', name: 'renamed.pdf' } }
			]
		}
	];

	const METHOD_TONE = { GET: 'ok', POST: 'warn', PUT: 'warn', DELETE: 'danger' };

	let openPath = $state(GROUPS[0].endpoints[0].path);
	const toggle = (p) => (openPath = openPath === p ? null : p);

	function sampleFor(e) {
		if (e.body) return JSON.stringify(e.body, null, 2);
		if (typeof e.sample === 'string') return e.sample;
		if (e.sample) return JSON.stringify(e.sample, null, 2);
		return '{\n  "status": 200,\n  "message": "OK",\n  "data": { }\n}';
	}

	function copyQuickStart() {
		navigator.clipboard.writeText(QUICK_START);
		toast.success('Copied', 'Paste it into a file and add your API key.');
	}
</script>

<Seo
	title="API documentation | Silocat"
	description="Integrate Silocat's zero-knowledge encrypted storage. The official client does the end-to-end encryption for you, so ciphertext is all that ever leaves your process."
	schema={breadcrumbSchema([
		{ name: 'Home', path: '/' },
		{ name: 'API', path: '/api' }
	])}
/>

<div class="page">
	<Navbar />

	<main class="main">
		<section class="head">
			<span class="eyebrow">developers</span>
			<h1>API</h1>
			<p class="sub">
				Everything the web app does, your code can do too. Encryption always happens on your side;
				the API only ever sees ciphertext.
			</p>
			<div class="base">
				<span class="base-label">base</span>
				<span class="base-url">https://silo.cat</span>
			</div>
		</section>

		<!-- Quick start -->
		<section class="block">
			<div class="block-head">
				<h2>Quick start</h2>
				<span class="block-sub">
					The official client handles the crypto. Start here unless you have a reason not to.
				</span>
			</div>

			<div class="install">
				<code>npm install @clickswave/silocat-client</code>
			</div>

			<div class="code">
				<div class="code-head">
					<span class="code-label">index.js</span>
					<button type="button" class="code-copy" onclick={copyQuickStart}>
						<Icon name="copy" size={13} /> Copy
					</button>
				</div>
				<pre>{QUICK_START}</pre>
			</div>
		</section>

		<!-- Auth -->
		<section class="block">
			<div class="block-head">
				<h2>Authentication</h2>
				<span class="block-sub">
					One key per account, sent as <code class="inline">X-Api-Key</code>.
				</span>
			</div>

			<div class="note-card">
				<p>
					Find your key in <a href="/home/settings">Settings → API</a>. It identifies your account
					outright: anything holding it can read, share and delete your files, so keep it in the
					same place you keep other secrets and never ship it to a browser.
				</p>
				<p>
					You can rotate it from the same screen. Rotation is immediate and total, with no overlap
					window, so anything still using the old key breaks until you paste the new one in.
				</p>
			</div>
		</section>

		<!-- Client reference -->
		<section class="block">
			<div class="block-head">
				<h2>Client reference</h2>
				<span class="block-sub">
					Errors throw <code class="inline">SilocatError</code> with <code class="inline">.status</code>
					and <code class="inline">.body</code>.
				</span>
			</div>

			<div class="methods">
				{#each METHODS as m (m.sig)}
					<div class="method">
						<code class="method-sig">{m.sig}</code>
						<span class="method-desc">{m.desc}</span>
					</div>
				{/each}
			</div>
		</section>

		<!-- Why the library -->
		<section class="block">
			<div class="block-head">
				<h2>Why there is a library</h2>
			</div>
			<div class="note-card">
				<p>
					Silocat is zero-knowledge, so the API cannot accept a file. It accepts ciphertext. Doing
					that by hand means deriving a key with Argon2id at libsodium's MODERATE limits, splitting
					the file into 100&nbsp;MB chunks, and encrypting each one with XChaCha20-Poly1305 under a
					fresh 24-byte nonce, with a single salt per file.
				</p>
				<p>
					Every one of those has to match exactly. Get a parameter wrong and the upload still
					succeeds, but the file can never be decrypted again, by you or by us. The library is the
					reference implementation, which is why the raw endpoints below are documented as a
					fallback rather than the recommended path.
				</p>
				<p class="note-foot">
					Two things no client can change: lose the password and the file is gone, and a share link
					never carries the key. Send the password through a different channel or the link alone is
					enough to read the file.
				</p>
			</div>
		</section>

		<!-- Raw HTTP -->
		<section class="block">
			<div class="block-head">
				<h2>Raw HTTP</h2>
				<span class="block-sub">
					Every route below is callable with your API key. Send it as
					<code class="inline">X-Api-Key</code>; responses are <code class="inline">application/json</code>.
				</span>
			</div>

			{#each GROUPS as group (group.title)}
				<div class="group">
					<div class="group-head">
						<h3>{group.title}</h3>
						<span class="group-desc">{group.desc}</span>
					</div>

					<div class="endpoints">
						{#each group.endpoints as e (e.path + e.method)}
							<div class="endpoint">
								<button
									type="button"
									class="row"
									aria-expanded={openPath === e.path}
									onclick={() => toggle(e.path)}
								>
									<span class="method {METHOD_TONE[e.method] || 'ok'}">{e.method}</span>
									<span class="path">{e.path}</span>
									<span class="desc">{e.desc}</span>
									<span class="chev" class:open={openPath === e.path}>
										<Icon name="chevron-down" size={15} />
									</span>
								</button>

								{#if openPath === e.path}
									<div class="detail" transition:slide={{ duration: 150 }}>
										<pre>{sampleFor(e)}</pre>
									</div>
								{/if}
							</div>
						{/each}
					</div>
				</div>
			{/each}
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
		max-width: 860px;
		margin: 0 auto;
		padding-inline: var(--gutter);
	}

	.head {
		padding: clamp(2.5rem, 7vw, 4rem) 0 var(--space-6);
		display: flex;
		flex-direction: column;
		gap: var(--space-3);

		h1 {
			margin: 0;
			font-size: var(--fs-h1);
			font-weight: var(--fw-black);
			letter-spacing: var(--tracking-tight);
			line-height: var(--lh-tight);
		}
	}

	.eyebrow {
		font-family: var(--font-mono);
		font-size: var(--fs-xs);
		color: var(--accent);
		text-transform: uppercase;
		letter-spacing: 0.1em;
	}

	.sub {
		margin: 0;
		max-width: 60ch;
		font-size: 1rem;
		color: var(--ink-mute);
	}

	.base {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		margin-top: var(--space-1);
		padding: 0.625rem 0.75rem;
		border: 1px solid var(--edge);
		border-radius: 8px;
		background: var(--surface);
		max-width: fit-content;
	}

	.base-label,
	.base-url {
		font-family: var(--font-mono);
		font-size: var(--fs-sm);
	}
	.base-label {
		color: var(--ink-faint);
	}

	/* ---- blocks ---- */
	.block {
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
		padding-bottom: 2.5rem;
	}

	.block-head {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);

		h2 {
			margin: 0;
			font-size: 1.25rem;
			font-weight: var(--fw-semibold);
			letter-spacing: var(--tracking-tight);
		}
	}

	.block-sub {
		font-size: var(--fs-sm);
		color: var(--ink-mute);
	}

	.install {
		padding: 0.75rem 0.875rem;
		border: 1px solid var(--edge);
		border-radius: var(--radius-md);
		background: var(--surface);

		code {
			font-family: var(--font-mono);
			font-size: var(--fs-sm);
			color: var(--ink);
		}
	}

	/* ---- code block ---- */
	.code {
		border: 1px solid var(--edge);
		border-radius: var(--radius-md);
		background: var(--surface);
		overflow: hidden;
	}

	.code-head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 0.5rem 0.875rem;
		border-bottom: 1px solid var(--edge);
	}

	.code-label {
		font-family: var(--font-mono);
		font-size: var(--fs-xs);
		color: var(--ink-faint);
	}

	.code-copy {
		display: inline-flex;
		align-items: center;
		gap: 0.375rem;
		border: 0;
		background: none;
		font: inherit;
		font-size: var(--fs-xs);
		color: var(--ink-mute);
		cursor: pointer;
		padding: 0.25rem 0.375rem;
		border-radius: var(--radius-sm);
		transition:
			background var(--dur-fast) var(--ease),
			color var(--dur-fast) var(--ease);

		&:hover {
			background: var(--tint-soft);
			color: var(--ink);
		}
	}

	pre {
		margin: 0;
		padding: 0.875rem;
		font-family: var(--font-mono);
		font-size: var(--fs-xs);
		line-height: 1.7;
		color: var(--ink-mute);
		overflow-x: auto;
	}

	.code pre {
		background: var(--bg);
	}

	/* ---- note card ---- */
	.note-card {
		display: flex;
		flex-direction: column;
		gap: 0.875rem;
		padding: 1.125rem 1.25rem;
		border: 1px solid var(--edge);
		border-radius: var(--radius-md);
		background: var(--surface);

		p {
			margin: 0;
			font-size: var(--fs-sm);
			color: var(--ink-mute);
			line-height: var(--lh-normal);
		}
	}

	.note-foot {
		padding-top: 0.875rem;
		border-top: 1px solid var(--edge);
	}

	code.inline {
		font-family: var(--font-mono);
		font-size: 0.9em;
		color: var(--ink);
	}

	/* ---- client reference ---- */
	.methods {
		border: 1px solid var(--edge);
		border-radius: var(--radius-md);
		background: var(--surface);
		overflow: hidden;
	}

	.method {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
		padding: 0.75rem 1rem;
		border-bottom: 1px solid var(--edge);

		&:last-child {
			border-bottom: 0;
		}
	}

	.method-sig {
		font-family: var(--font-mono);
		font-size: var(--fs-sm);
		color: var(--ink);
	}

	.method-desc {
		font-size: var(--fs-xs);
		color: var(--ink-faint);
	}

	/* ---- raw http ---- */
	.group {
		display: flex;
		flex-direction: column;
		gap: 0.625rem;
		padding-bottom: var(--space-5);
	}

	.group-head {
		display: flex;
		flex-direction: column;
		gap: 0.125rem;

		h3 {
			margin: 0;
			font-size: 0.9375rem;
			font-weight: var(--fw-semibold);
			letter-spacing: var(--tracking-tight);
		}
	}

	.group-desc {
		font-size: var(--fs-sm);
		color: var(--ink-mute);
	}

	.endpoints {
		border: 1px solid var(--edge);
		border-radius: var(--radius-md);
		background: var(--surface);
		overflow: hidden;
	}

	.endpoint {
		border-bottom: 1px solid var(--edge);

		&:last-child {
			border-bottom: 0;
		}
	}

	.row {
		width: 100%;
		display: flex;
		align-items: center;
		gap: 0.875rem;
		padding: 0.75rem 1rem;
		border: 0;
		background: none;
		text-align: left;
		font: inherit;
		color: inherit;
		cursor: pointer;
		transition: background var(--dur-fast) var(--ease);

		&:hover {
			background: var(--surface-hover);
		}
	}

	.method {
		&.ok,
		&.warn,
		&.danger {
			flex: 0 0 52px;
			display: inline-flex;
			align-items: center;
			justify-content: center;
			height: 22px;
			padding: 0;
			border: 0;
			border-radius: var(--radius-sm);
			font-family: var(--font-mono);
			font-size: 0.6875rem;
			font-weight: var(--fw-semibold);
		}
		&.ok {
			background: var(--ok-soft);
			color: var(--ok);
		}
		&.warn {
			background: var(--warn-soft);
			color: var(--warn);
		}
		&.danger {
			background: var(--danger-soft);
			color: var(--danger);
		}
	}

	.path {
		flex: 0 0 auto;
		font-family: var(--font-mono);
		font-size: var(--fs-sm);
	}

	.desc {
		flex: 1;
		min-width: 0;
		font-size: var(--fs-sm);
		color: var(--ink-faint);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.chev {
		flex: 0 0 auto;
		color: var(--ink-faint);
		display: grid;
		place-items: center;
		transition: transform var(--dur) var(--ease);

		&.open {
			transform: rotate(180deg);
		}
	}

	.detail {
		padding: 0 1rem 1rem;

		pre {
			border-radius: 8px;
			background: var(--bg);
			border: 1px solid var(--edge);
		}
	}

	@media (max-width: 720px) {
		.row {
			flex-wrap: wrap;
			gap: var(--space-2);
		}
		.desc {
			flex: 1 0 100%;
			white-space: normal;
		}
	}
</style>
