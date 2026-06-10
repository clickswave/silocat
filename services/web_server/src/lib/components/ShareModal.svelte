<script>
	import { createEventDispatcher } from 'svelte';
	import { slide } from 'svelte/transition';
	import Icon from '@iconify/svelte';
	import axios from 'axios';
	import { toast } from 'svelte-sonner';

	export let item; // The file or folder object { id, name, type, ... }

	const dispatch = createEventDispatcher();

	// State
	let loading = true;
	let shareType = 'off'; // 'off', 'public', 'once'
	let shareToken = null;
	let downloads = 0;
	let maxDownloads = 1;
	let link = '';

	// Hardening
	let expiresAt = null; // ISO string or null
	let passwordProtected = false; // current saved state
	let expiryChoice = '0'; // '0' = never, else days
	let newPassword = ''; // new/changed password to apply
	let removePassword = false;
	let savingOptions = false;

	function expiryLabel(iso) {
		if (!iso) return null;
		const d = new Date(iso);
		const now = new Date();
		if (d < now) return 'Expired';
		return 'Expires ' + d.toLocaleString();
	}

	// Fetch initial status
	import { onMount } from 'svelte';
	onMount(async () => {
		console.log('ShareModal: Mounted with item', item);
		if (!item || !item.id) {
			console.error('ShareModal: Invalid item', item);
			return;
		}
		try {
			console.log('ShareModal: Fetching info for', item.id);
			const res = await axios.get(
				`/api/v1/sanctum/file/share/info/${item.id}?user_id=${window.currentUser?.id || ''}`
			);
			// Note: user_id might be needed if not in session?
			// Actually, the proxy handles the session and backend expects query param or extension.
			// My proxy `info/[id]/+server.js` forwarding logic might need checking if it passes user_id?

			if (res.data.success) {
				console.log('ShareModal: Info received', res.data.success.data);
				const data = res.data.success.data;
				shareType = data.share_type || 'off';
				shareToken = data.share_token;
				downloads = data.link_downloads || 0;
				maxDownloads = data.link_max_downloads || 1;
				expiresAt = data.expires_at || null;
				passwordProtected = !!data.password_protected;
				updateLink();
			}
		} catch (e) {
			console.error('ShareModal Error:', e);
			// Default to off if not found or error
		} finally {
			loading = false;
		}
	});

	function updateLink() {
		if (shareToken) {
			link = `${window.location.origin}/s/${shareToken}`;
		} else {
			link = '';
		}
	}

	function targetField(payload) {
		if (item.type === 'folder' || item.name === 'Folder') payload.folder_id = item.id;
		else payload.file_id = item.id;
		return payload;
	}

	function applyResponse(data) {
		shareToken = data.share_token;
		downloads = data.link_downloads;
		maxDownloads = data.link_max_downloads;
		expiresAt = data.expires_at || null;
		passwordProtected = !!data.password_protected;
		updateLink();
	}

	// Toggle only changes share_type; expiry/password are left untouched server-side.
	async function handleToggle(type) {
		if (loading) return;
		const oldType = shareType;
		shareType = type;
		try {
			const res = await axios.post(
				'/api/v1/sanctum/file/share/toggle',
				targetField({ share_type: type })
			);
			if (res.data.success) {
				applyResponse(res.data.success.data);
				toast.success('Share settings updated');
			} else {
				shareType = oldType;
				toast.error('Failed to update settings');
			}
		} catch (e) {
			shareType = oldType;
			toast.error('Error updating settings');
			console.error(e);
		}
	}

	// Apply expiry + password options to the current share.
	async function applyOptions() {
		if (loading || savingOptions) return;
		savingOptions = true;
		try {
			const payload = targetField({ share_type: shareType });
			payload.expires_in_days = parseInt(expiryChoice, 10) || 0;
			if (removePassword) payload.clear_password = true;
			else if (newPassword.trim()) payload.password = newPassword.trim();

			const res = await axios.post('/api/v1/sanctum/file/share/toggle', payload);
			if (res.data.success) {
				applyResponse(res.data.success.data);
				newPassword = '';
				removePassword = false;
				toast.success('Link options saved');
			} else {
				toast.error('Failed to save options');
			}
		} catch (e) {
			toast.error('Error saving options');
			console.error(e);
		} finally {
			savingOptions = false;
		}
	}

	async function handleRegenerate() {
		if (!shareToken) return;
		try {
			const payload = {};
			if (item.type === 'folder' || item.name === 'Folder') {
				payload.folder_id = item.id;
			} else {
				payload.file_id = item.id;
			}

			const res = await axios.post('/api/v1/sanctum/file/share/regenerate', payload);
			if (res.data.success) {
				shareToken = res.data.success.data.share_token;
				downloads = 0; // Reset downloads on regen
				updateLink();
				toast.success('Link regenerated');
			}
		} catch (e) {
			toast.error('Failed to regenerate link');
		}
	}

	function copyLink() {
		navigator.clipboard.writeText(link);
		toast.success('Link copied to clipboard');
	}

	function close() {
		dispatch('close');
	}
</script>

<div class="modal-backdrop" on:click={close}>
	<div class="modal" on:click|stopPropagation>
		<div class="modal-header">
			<div class="head-title">
				<span class="title-icon"><Icon icon="ri:share-forward-line" width="20" /></span>
				<h3>Share “{item.name}”</h3>
			</div>
			<button class="close-btn" on:click={close} aria-label="Close">
				<Icon icon="ri:close-line" width="22" />
			</button>
		</div>

		<div class="modal-body">
			{#if loading}
				<div class="loading">
					<Icon icon="ri:loader-4-line" class="spinner" width="32" />
				</div>
			{:else}
				<div class="section toggle-section">
					<div class="option-row">
						<div class="info">
							<span class="label">Share access</span>
							<span class="desc">Allow others to access this item</span>
						</div>
						<div class="toggles">
							<button
								class="toggle-btn {shareType === 'off' ? 'active' : ''}"
								on:click={() => handleToggle('off')}
							>
								Off
							</button>
							<button
								class="toggle-btn {shareType === 'public' ? 'active' : ''}"
								on:click={() => handleToggle('public')}
							>
								Public
							</button>
							<button
								class="toggle-btn {shareType === 'once' ? 'active' : ''}"
								on:click={() => handleToggle('once')}
							>
								Once
							</button>
						</div>
					</div>
				</div>

				{#if shareType !== 'off'}
					<div class="section link-section" transition:slide>
						<div class="input-group">
							<input type="text" readonly value={link} />
							<div class="actions">
								<button class="action-btn copy" on:click={copyLink} title="Copy Link">
									<Icon icon="ri:file-copy-line" width="20" />
								</button>
								<button
									class="action-btn regen"
									on:click={handleRegenerate}
									title="Regenerate Link"
								>
									<Icon icon="ri:refresh-line" width="20" />
								</button>
							</div>
						</div>
						<div class="stats">
							<Icon icon="ri:bar-chart-box-line" width="16" />
							{#if shareType === 'once'}
								<span>{downloads} / {maxDownloads} downloads used</span>
							{:else}
								<span>{downloads} {downloads === 1 ? 'download' : 'downloads'} so far</span>
							{/if}
							{#if passwordProtected}
								<span class="badge"><Icon icon="ri:lock-2-line" width="12" /> Password</span>
							{/if}
							{#if expiresAt}
								<span class="badge {new Date(expiresAt) < new Date() ? 'danger' : ''}">
									<Icon icon="ri:time-line" width="12" /> {expiryLabel(expiresAt)}
								</span>
							{/if}
						</div>

						<!-- Link hardening options -->
						<div class="opts">
							<label class="opt-field">
								<span class="opt-label">Link expiry</span>
								<select bind:value={expiryChoice}>
									<option value="0">Never</option>
									<option value="1">1 day</option>
									<option value="7">7 days</option>
									<option value="30">30 days</option>
									<option value="90">90 days</option>
								</select>
							</label>

							<label class="opt-field">
								<span class="opt-label">
									Password {#if passwordProtected}<span class="set-tag">set</span>{/if}
								</span>
								<input
									type="password"
									placeholder={passwordProtected ? 'Enter new password' : 'Optional password'}
									bind:value={newPassword}
									autocomplete="new-password"
									disabled={removePassword}
								/>
							</label>

							{#if passwordProtected}
								<label class="opt-remove">
									<input type="checkbox" bind:checked={removePassword} />
									<span>Remove password</span>
								</label>
							{/if}

							<button class="save-opts" on:click={applyOptions} disabled={savingOptions}>
								{savingOptions ? 'Saving…' : 'Save options'}
							</button>
						</div>
					</div>
				{/if}
			{/if}
		</div>
	</div>
</div>

<style lang="scss">
	.modal-backdrop {
		position: fixed;
		top: 0;
		left: 0;
		width: 100vw;
		height: 100vh;
		background: rgba(0, 0, 0, 0.65);
		backdrop-filter: blur(8px);
		z-index: 1000;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: var(--gutter);
	}

	.modal {
		background: var(--bg-elevated);
		border: 1px solid var(--border-default);
		border-radius: var(--radius-lg);
		width: 100%;
		max-width: 480px;
		max-height: 90vh;
		display: flex;
		flex-direction: column;
		box-shadow: var(--shadow-lg);
		overflow: hidden;
		animation: slideUp 0.3s cubic-bezier(0.16, 1, 0.3, 1);
	}

	.modal-header {
		padding: var(--space-5);
		border-bottom: 1px solid var(--hairline);
		display: flex;
		justify-content: space-between;
		align-items: center;
		gap: var(--space-3);
		flex-shrink: 0;

		.head-title {
			display: flex;
			align-items: center;
			gap: var(--space-3);
			min-width: 0;
		}
		.title-icon {
			display: grid;
			place-items: center;
			width: 34px;
			height: 34px;
			flex-shrink: 0;
			border-radius: var(--radius-md);
			background: var(--tint-soft);
			color: var(--primary);
		}

		h3 {
			margin: 0;
			font-size: var(--fs-lg);
			font-weight: var(--fw-semibold);
			color: var(--text-primary);
			overflow: hidden;
			text-overflow: ellipsis;
			white-space: nowrap;
		}

		.close-btn {
			background: transparent;
			border: none;
			color: var(--text-muted);
			cursor: pointer;
			padding: var(--space-1);
			border-radius: var(--radius-sm);
			display: flex;
			transition: color var(--dur) var(--ease), background var(--dur) var(--ease);

			&:hover {
				color: var(--text-primary);
				background: var(--tint-softer);
			}
		}
	}

	.modal-body {
		padding: var(--space-6);
		display: flex;
		flex-direction: column;
		gap: var(--space-5);
		flex: 1;
		min-height: 0;
		overflow-y: auto;

		.loading {
			display: flex;
			justify-content: center;
			padding: var(--space-5);

			.spinner {
				animation: spin 1s linear infinite;
				color: var(--primary);
			}
		}
	}

	.section {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
	}

	.option-row {
		display: flex;
		justify-content: space-between;
		align-items: center;
		gap: var(--space-4);

		.info {
			display: flex;
			flex-direction: column;
			gap: var(--space-1);

			.label {
				color: var(--text-primary);
				font-weight: var(--fw-medium);
			}
			.desc {
				font-size: var(--fs-sm);
				color: var(--text-muted);
			}
		}

		.toggles {
			display: flex;
			background: var(--tint-soft);
			border: 1px solid var(--border-default);
			padding: var(--space-1);
			border-radius: var(--radius-md);
			gap: var(--space-1);

			.toggle-btn {
				background: transparent;
				border: none;
				color: var(--text-secondary);
				padding: var(--space-2) var(--space-3);
				border-radius: var(--radius-sm);
				font-family: inherit;
				font-size: var(--fs-sm);
				cursor: pointer;
				transition: background var(--dur) var(--ease), color var(--dur) var(--ease);
				font-weight: var(--fw-medium);

				&.active {
					background: var(--accent-gradient);
					color: #fff;
					box-shadow: 0 4px 12px -4px var(--primary-glow);
				}

				&:hover:not(.active) {
					color: var(--text-primary);
				}
			}
		}
	}

	.input-group {
		display: flex;
		gap: var(--space-2);

		input {
			flex: 1;
			background: var(--bg-input);
			border: 1px solid var(--border-default);
			border-radius: var(--radius-sm);
			padding: 0.75rem 0.95rem;
			color: var(--text-secondary);
			font-size: var(--fs-sm);
			font-family: var(--font-mono);

			&:focus {
				outline: none;
				border-color: var(--primary);
				box-shadow: 0 0 0 3px var(--primary-glow);
			}
		}

		.actions {
			display: flex;
			gap: var(--space-1);
		}

		.action-btn {
			background: var(--tint-soft);
			border: 1px solid var(--border-default);
			color: var(--text-secondary);
			width: 42px;
			border-radius: var(--radius-sm);
			cursor: pointer;
			display: flex;
			align-items: center;
			justify-content: center;
			transition: background var(--dur) var(--ease), color var(--dur) var(--ease),
				border-color var(--dur) var(--ease);

			&:hover {
				background: var(--tint-softer);
				color: var(--text-primary);
			}

			&.copy {
				&:hover {
					color: var(--success);
					border-color: var(--success);
					background: rgba(61, 220, 151, 0.1);
				}
			}
			&.regen {
				&:hover {
					color: var(--warning);
					border-color: var(--warning);
					background: rgba(242, 201, 76, 0.1);
				}
			}
		}
	}

	.stats {
		display: flex;
		align-items: center;
		flex-wrap: wrap;
		gap: var(--space-2);
		font-size: var(--fs-sm);
		color: var(--text-muted);
		padding-left: var(--space-1);

		.badge {
			display: inline-flex;
			align-items: center;
			gap: 3px;
			font-size: var(--fs-xs);
			background: var(--tint-soft);
			border: 1px solid var(--border-default);
			color: var(--text-secondary);
			padding: 2px 7px;
			border-radius: 999px;
			&.danger {
				color: var(--danger);
				border-color: var(--danger);
			}
		}
	}

	.opts {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
		margin-top: var(--space-4);
		padding-top: var(--space-4);
		border-top: 1px solid var(--hairline);

		.opt-field {
			display: flex;
			flex-direction: column;
			gap: var(--space-1);

			.opt-label {
				font-size: var(--fs-xs);
				color: var(--text-muted);
				font-weight: var(--fw-medium);
				display: flex;
				align-items: center;
				gap: var(--space-2);
			}
			.set-tag {
				font-size: 0.65rem;
				text-transform: uppercase;
				letter-spacing: 0.04em;
				color: var(--success);
				background: rgba(61, 220, 151, 0.12);
				padding: 1px 6px;
				border-radius: 999px;
			}
			select,
			input {
				background: var(--bg-input);
				border: 1px solid var(--border-default);
				border-radius: var(--radius-sm);
				padding: 0.6rem 0.75rem;
				color: var(--text-primary);
				font-family: inherit;
				font-size: var(--fs-sm);
				outline: none;
				transition: border-color var(--dur) var(--ease), box-shadow var(--dur) var(--ease);
				&:focus {
					border-color: var(--primary);
					box-shadow: 0 0 0 3px var(--primary-glow);
				}
				&:disabled {
					opacity: 0.5;
				}
			}
		}
		.opt-remove {
			display: flex;
			align-items: center;
			gap: var(--space-2);
			font-size: var(--fs-sm);
			color: var(--text-secondary);
			cursor: pointer;
			input {
				accent-color: var(--primary);
			}
		}
		.save-opts {
			align-self: flex-start;
			background: var(--primary);
			color: #fff;
			border: none;
			border-radius: var(--radius-sm);
			padding: var(--space-2) var(--space-4);
			font-family: inherit;
			font-weight: var(--fw-medium);
			font-size: var(--fs-sm);
			cursor: pointer;
			&:hover {
				filter: brightness(1.05);
			}
			&:disabled {
				opacity: 0.6;
				cursor: default;
			}
		}
	}

	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
	}

	@keyframes slideUp {
		from {
			transform: translateY(20px);
			opacity: 0;
		}
		to {
			transform: translateY(0);
			opacity: 1;
		}
	}

	@media (max-width: 600px) {
		.modal-backdrop {
			align-items: flex-end;
			padding: 0;
		}
		.modal {
			max-width: 100%;
			max-height: 92vh;
			border-radius: var(--radius-lg) var(--radius-lg) 0 0;
		}
	}
</style>
