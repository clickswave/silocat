<script>
	import Navbar from '$lib/components/Navbar.svelte';
	import Footer from '$lib/components/Footer.svelte';
	import Icon from '@iconify/svelte';
	import { slide } from 'svelte/transition';

	let expandedEndpoint = $state(null);

	function toggleEndpoint(path) {
		if (expandedEndpoint === path) {
			expandedEndpoint = null;
		} else {
			expandedEndpoint = path;
		}
	}

	const apiGroups = [
		{
			title: 'Public APIs',
			description: 'Open endpoints for file sharing and general access.',
			icon: 'ri:earth-line',
			endpoints: [
				{
					method: 'POST',
					path: '/api/v1/public/share/authorize',
					desc: 'Authorize access to a shared file via password.',
					body: {
						token: 'share-token-uuid',
						password: 'secret-password'
					}
				},
				{
					method: 'GET',
					path: '/api/v1/public/share/info/[token]',
					desc: 'Get metadata for a shared file using its token.'
				},
				{
					method: 'GET',
					path: '/api/v1/public/share/fetch-chunks',
					desc: 'Download file chunks for public shared files.'
				}
			]
		},
		{
			title: 'Sanctum APIs',
			description: 'Authenticated endpoints for user file management. Requires Session Cookie.',
			icon: 'ri:shield-user-line',
			endpoints: [
				{
					method: 'POST',
					path: '/api/v1/sanctum/file',
					desc: 'Upload a new file.',
					body: {
						file_name: 'example.txt',
						file_size: 1024,
						file_type: 'text/plain',
						total_chunks: 1,
						folder_id: 'optional-uuid'
					}
				},
				{
					method: 'GET',
					path: '/api/v1/sanctum/file/list',
					desc: 'List files (supports folder_id, starred, shared filters).'
				},
				{
					method: 'POST',
					path: '/api/v1/sanctum/file/delete',
					desc: 'Move files to trash.',
					body: {
						file_ids: ['uuid-1', 'uuid-2']
					}
				},
				{
					method: 'POST',
					path: '/api/v1/sanctum/file/permanent-delete',
					desc: 'Permanently delete files from trash.',
					body: {
						file_ids: ['uuid-1', 'uuid-2']
					}
				},
				{
					method: 'POST',
					path: '/api/v1/sanctum/file/restore',
					desc: 'Restore files from trash.',
					body: {
						file_ids: ['uuid-1', 'uuid-2']
					}
				},
				{
					method: 'POST',
					path: '/api/v1/sanctum/file/star',
					desc: 'Toggle star status for a file.',
					body: {
						file_id: 'uuid',
						starred: true
					}
				},
				{
					method: 'POST',
					path: '/api/v1/sanctum/file/update',
					desc: 'Rename or update file metadata.',
					body: {
						file_id: 'uuid',
						file_name: 'new-name.txt'
					}
				},
				{
					method: 'GET',
					path: '/api/v1/sanctum/file/fetch-chunks',
					desc: 'Download authenticated file chunks.'
				},
				{
					method: 'POST',
					path: '/api/v1/sanctum/file/mark-chunk-complete',
					desc: 'Mark a file chunk as uploaded.',
					body: {
						file_id: 'uuid',
						chunk_index: 0
					}
				},
				{
					method: 'POST',
					path: '/api/v1/sanctum/file/share/toggle',
					desc: 'Enable or disable file sharing.',
					body: {
						file_id: 'uuid',
						shared: true
					}
				},
				{
					method: 'GET',
					path: '/api/v1/sanctum/file/share/info/[id]',
					desc: 'Get sharing details for a file.'
				},
				{
					method: 'POST',
					path: '/api/v1/sanctum/file/share/regenerate',
					desc: 'Regenerate public share token.',
					body: {
						file_id: 'uuid'
					}
				},
				{
					method: 'POST',
					path: '/api/v1/sanctum/folder/create',
					desc: 'Create a new folder.',
					body: {
						folder_name: 'My Documents',
						parent_folder_id: 'optional-uuid'
					}
				},
				{ method: 'GET', path: '/api/v1/sanctum/folder/list', desc: 'List folders.' },
				{
					method: 'POST',
					path: '/api/v1/sanctum/folder/delete',
					desc: 'Move folder to trash.',
					body: {
						folder_id: 'uuid'
					}
				},
				{
					method: 'POST',
					path: '/api/v1/sanctum/folder/permanent-delete',
					desc: 'Permanently delete folder.',
					body: {
						folder_id: 'uuid'
					}
				},
				{
					method: 'POST',
					path: '/api/v1/sanctum/folder/restore',
					desc: 'Restore folder from trash.',
					body: {
						folder_id: 'uuid'
					}
				},
				{
					method: 'POST',
					path: '/api/v1/sanctum/folder/star',
					desc: 'Toggle star status for a folder.',
					body: {
						folder_id: 'uuid',
						starred: true
					}
				},
				{
					method: 'POST',
					path: '/api/v1/sanctum/folder/update',
					desc: 'Rename folder.',
					body: {
						folder_id: 'uuid',
						folder_name: 'New Name'
					}
				},
				{
					method: 'GET',
					path: '/api/v1/sanctum/user/storage',
					desc: 'Get user storage usage stats.'
				},
				{
					method: 'POST',
					path: '/api/v1/sanctum/user/update-profile',
					desc: 'Update user profile settings.',
					body: {
						username: 'new_handle',
						avatar_url: 'https://...'
					}
				}
			]
		},
		{
			title: 'Shadow APIs',
			description: 'Anonymous operations using Browser/Shadow Key identity.',
			icon: 'ri:spy-line',
			endpoints: [
				{
					method: 'POST',
					path: '/api/v1/shadow/file',
					desc: 'Upload file as anonymous user.',
					body: {
						file_name: 'secret.txt',
						file_size: 512,
						file_type: 'text/plain',
						total_chunks: 1
					}
				},
				{
					method: 'POST',
					path: '/api/v1/shadow/file/fetch-files',
					desc: 'List anonymous files by API key.',
					body: {
						api_key: 'shadow-key-uuid'
					}
				},
				{
					method: 'POST',
					path: '/api/v1/shadow/file/delete',
					desc: 'Delete anonymous file.',
					body: {
						file_id: 'uuid',
						api_key: 'shadow-key-uuid'
					}
				},
				{
					method: 'GET',
					path: '/api/v1/shadow/file/fetch-chunks',
					desc: 'Download anonymous file chunks.'
				},
				{
					method: 'POST',
					path: '/api/v1/shadow/file/mark-chunk-complete',
					desc: 'Mark anonymous chunk complete.',
					body: {
						file_id: 'uuid',
						chunk_index: 0,
						api_key: 'shadow-key-uuid'
					}
				},
				{
					method: 'POST',
					path: '/api/v1/shadow/folder',
					desc: 'Create anonymous folder.',
					body: {
						folder_name: 'Anon Folder',
						api_key: 'shadow-key-uuid'
					}
				},
				{
					method: 'POST',
					path: '/api/v1/shadow/folder/fetch',
					desc: 'List anonymous folders.',
					body: {
						api_key: 'shadow-key-uuid'
					}
				},
				{
					method: 'POST',
					path: '/api/v1/shadow/folder/delete',
					desc: 'Delete anonymous folder.',
					body: {
						folder_id: 'uuid',
						api_key: 'shadow-key-uuid'
					}
				},
				{ method: 'GET', path: '/api/v1/shadow/resource/fetch', desc: 'Fetch resource metadata.' }
			]
		}
	];
</script>

<svelte:head>
	<title>API Documentation | SiloCat</title>
</svelte:head>

<div class="api-page">
	<Navbar />

	<main class="content">
		<div class="header">
			<h1>API Documentation</h1>
			<p>Complete reference for SiloCat v1 REST API endpoints.</p>
		</div>

		<div class="grid">
			{#each apiGroups as group}
				<div class="group-card">
					<div class="group-header">
						<div class="icon-wrapper">
							<Icon icon={group.icon} width="24" />
						</div>
						<div class="text">
							<h2>{group.title}</h2>
							<p>{group.description}</p>
						</div>
					</div>

					<div class="endpoint-list">
						{#each group.endpoints as endpoint}
							<button
								class="endpoint-row"
								class:expanded={expandedEndpoint === endpoint.path}
								onclick={() => toggleEndpoint(endpoint.path)}
							>
								<div class="row-content">
									<span class="method {endpoint.method.toLowerCase()}">{endpoint.method}</span>
									<div class="details">
										<code class="path">{endpoint.path}</code>
										<span class="desc">{endpoint.desc}</span>
									</div>
								</div>
								<Icon
									icon="ri:arrow-down-s-line"
									width="20"
									class="chevron"
									style="transform: {expandedEndpoint === endpoint.path
										? 'rotate(180deg)'
										: 'rotate(0deg)'}; transition: transform 0.2s;"
								/>
							</button>

							{#if expandedEndpoint === endpoint.path}
								<div class="endpoint-details" transition:slide={{ duration: 200 }}>
									<div class="detail-block">
										<span class="label">Authentication</span>
										<span class="value">
											{#if group.title === 'Public APIs'}
												None (or Token verified)
											{:else if group.title === 'Shadow APIs'}
												Shadow Key (Header: X-Shadow-Key)
											{:else}
												Session Cookie (Authenticated)
											{/if}
										</span>
									</div>
									<div class="detail-block">
										<span class="label">Response Format</span>
										<code class="value">application/json</code>
									</div>

									{#if endpoint.body}
										<div class="detail-block">
											<span class="label">Sample Request Body</span>
											<pre class="json-block">{JSON.stringify(endpoint.body, null, 2)}</pre>
										</div>
									{/if}
								</div>
							{/if}
						{/each}
					</div>
				</div>
			{/each}
		</div>
	</main>

	<Footer />
</div>

<style lang="scss">
	.api-page {
		min-height: 100vh;
		background: var(--bg-app);
		display: flex;
		flex-direction: column;
	}

	.content {
		flex: 1;
		width: 100%;
		max-width: 1200px;
		margin: 0 auto;
		padding: 3rem 1.5rem;
		display: flex;
		flex-direction: column;
		gap: 3rem;
	}

	.header {
		text-align: center;
		padding: 2rem 0;

		h1 {
			font-size: 2.5rem;
			font-weight: 800;
			margin: 0 0 1rem 0;
			background: linear-gradient(135deg, #fff 0%, #a1a1aa 100%);
			background-clip: text;
			-webkit-background-clip: text;
			-webkit-text-fill-color: transparent;
			letter-spacing: -0.02em;
		}

		p {
			color: var(--text-muted);
			font-size: 1.1rem;
			max-width: 600px;
			margin: 0 auto;
		}
	}

	.grid {
		display: grid;
		grid-template-columns: 1fr;
		gap: 2rem;
	}

	.group-card {
		background: var(--bg-card);
		border: 1px solid var(--border-default);
		border-radius: 20px;
		padding: 0;
		overflow: hidden;

		.group-header {
			padding: 1.5rem 2rem;
			border-bottom: 1px solid var(--border-default);
			display: flex;
			align-items: center;
			gap: 1.5rem;
			background: rgba(255, 255, 255, 0.02);

			.icon-wrapper {
				width: 48px;
				height: 48px;
				background: var(--bg-input);
				border-radius: 12px;
				display: flex;
				align-items: center;
				justify-content: center;
				color: var(--primary, #ff4655);
				border: 1px solid var(--border-default);
			}

			.text {
				h2 {
					margin: 0;
					font-size: 1.25rem;
					color: var(--text-primary);
					font-weight: 600;
				}

				p {
					margin: 0.25rem 0 0 0;
					color: var(--text-muted);
					font-size: 0.9rem;
				}
			}
		}

		.endpoint-list {
			display: flex;
			flex-direction: column;
		}

		.endpoint-row {
			display: flex;
			align-items: center;
			justify-content: space-between;
			gap: 1.5rem;
			padding: 1rem 2rem;
			border: none;
			border-bottom: 1px solid rgba(255, 255, 255, 0.03);
			transition: background 0.2s;
			background: transparent;
			width: 100%;
			text-align: left;
			cursor: pointer;

			&:last-child {
				border-bottom: none;
			}

			&:hover {
				background: var(--bg-card-hover);
			}

			&.expanded {
				background: rgba(255, 255, 255, 0.03);
			}

			.row-content {
				display: flex;
				align-items: flex-start;
				gap: 1.5rem;
				flex: 1;
			}

			.chevron {
				color: var(--text-muted);
				flex-shrink: 0;
			}

			.method {
				font-size: 0.75rem;
				font-weight: 700;
				padding: 0.25rem 0.5rem;
				border-radius: 6px;
				min-width: 60px;
				text-align: center;
				text-transform: uppercase;
				letter-spacing: 0.05em;
				margin-top: 2px;

				&.get {
					background: rgba(61, 220, 151, 0.15);
					color: #3ddc97;
					border: 1px solid rgba(61, 220, 151, 0.3);
				}
				&.post {
					background: rgba(255, 193, 7, 0.15);
					color: #ffc107;
					border: 1px solid rgba(255, 193, 7, 0.3);
				}
				&.put,
				&.patch {
					background: rgba(74, 163, 226, 0.15);
					color: #4aa3e2;
					border: 1px solid rgba(74, 163, 226, 0.3);
				}
				&.delete {
					background: rgba(255, 70, 85, 0.15);
					color: #ff4655;
					border: 1px solid rgba(255, 70, 85, 0.3);
				}
			}

			.details {
				display: flex;
				flex-direction: column;
				gap: 0.25rem;
				flex: 1;

				.path {
					color: var(--text-primary);
					font-family: 'JetBrains Mono', monospace;
					font-size: 0.9rem;
				}

				.desc {
					color: var(--text-muted);
					font-size: 0.9rem;
				}
			}
		}

		.endpoint-details {
			background: rgba(0, 0, 0, 0.2);
			padding: 1.5rem 2rem;
			border-bottom: 1px solid rgba(255, 255, 255, 0.03);
			display: flex;
			flex-direction: column;
			gap: 1rem;
			box-shadow: inset 0 4px 6px rgba(0, 0, 0, 0.1);

			.detail-block {
				display: flex;
				flex-direction: column;
				gap: 0.5rem;

				.label {
					font-size: 0.8rem;
					font-weight: 600;
					color: var(--text-muted);
					text-transform: uppercase;
					letter-spacing: 0.05em;
				}

				.value {
					font-size: 0.95rem;
					color: var(--text-primary);
					font-family: inherit;
				}

				code.value {
					font-family: 'JetBrains Mono', monospace;
					color: var(--primary, #ff4655);
				}

				.json-block {
					background: var(--bg-input);
					padding: 1rem;
					border-radius: 8px;
					border: 1px solid var(--border-default);
					margin: 0;
					font-family: 'JetBrains Mono', monospace;
					font-size: 0.85rem;
					color: #a1a1aa;
					overflow-x: auto;
					white-space: pre-wrap;
				}
			}
		}
	}

	@media (max-width: 768px) {
		.group-header {
			flex-direction: column;
			align-items: flex-start;
			gap: 1rem;

			.icon-wrapper {
				width: 40px;
				height: 40px;
			}
		}

		.endpoint-row {
			flex-direction: column;
			gap: 0.75rem;

			.method {
				align-self: flex-start;
			}
		}
	}
</style>
