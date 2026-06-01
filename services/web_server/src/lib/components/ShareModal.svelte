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

	async function handleToggle(type) {
		if (loading) return;
		const oldType = shareType;
		shareType = type;

		try {
			const payload = {
				share_type: type
			};
			if (item.type === 'folder' || item.name === 'Folder') {
				// Basic check, better if props are strict
				payload.folder_id = item.id;
			} else {
				payload.file_id = item.id;
			}

			const res = await axios.post('/api/v1/sanctum/file/share/toggle', payload);
			if (res.data.success) {
				const data = res.data.success.data;
				shareToken = data.share_token;
				downloads = data.link_downloads;
				maxDownloads = data.link_max_downloads;
				updateLink();
				toast.success('Share settings updated');
			} else {
				shareType = oldType; // Revert on fail
				toast.error('Failed to update settings');
			}
		} catch (e) {
			shareType = oldType;
			toast.error('Error updating settings');
			console.error(e);
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
			<h3>Share "{item.name}"</h3>
			<button class="close-btn" on:click={close}>
				<Icon icon="ri:close-line" width="24" />
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
						{#if shareType === 'once'}
							<div class="stats">
								<Icon icon="ri:time-line" width="16" />
								<span>{downloads} / {maxDownloads} downloads used</span>
							</div>
						{/if}
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
		background: rgba(0, 0, 0, 0.6);
		backdrop-filter: blur(4px);
		z-index: 1000;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.modal {
		background: #18181b;
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: 16px;
		width: 90%;
		max-width: 480px;
		box-shadow: 0 24px 48px rgba(0, 0, 0, 0.5);
		overflow: hidden;
	}

	.modal-header {
		padding: 20px;
		border-bottom: 1px solid rgba(255, 255, 255, 0.05);
		display: flex;
		justify-content: space-between;
		align-items: center;

		h3 {
			margin: 0;
			font-size: 18px;
			font-weight: 600;
			color: #eee;
		}

		.close-btn {
			background: transparent;
			border: none;
			color: #999;
			cursor: pointer;
			padding: 4px;
			border-radius: 4px;

			&:hover {
				color: #fff;
				background: rgba(255, 255, 255, 0.1);
			}
		}
	}

	.modal-body {
		padding: 24px;
		display: flex;
		flex-direction: column;
		gap: 24px;

		.loading {
			display: flex;
			justify-content: center;
			padding: 20px;

			.spinner {
				animation: spin 1s linear infinite;
				color: var(--primary, #ff4655);
			}
		}
	}

	.section {
		display: flex;
		flex-direction: column;
		gap: 12px;
	}

	.option-row {
		display: flex;
		justify-content: space-between;
		align-items: center;
		gap: 16px;

		.info {
			display: flex;
			flex-direction: column;
			gap: 4px;

			.label {
				color: #eee;
				font-weight: 500;
			}
			.desc {
				font-size: 13px;
				color: #888;
			}
		}

		.toggles {
			display: flex;
			background: rgba(255, 255, 255, 0.05);
			padding: 4px;
			border-radius: 8px;
			gap: 2px;

			.toggle-btn {
				background: transparent;
				border: none;
				color: #888;
				padding: 6px 12px;
				border-radius: 6px;
				font-size: 13px;
				cursor: pointer;
				transition: all 0.2s;
				font-weight: 500;

				&.active {
					background: rgba(255, 255, 255, 0.1);
					color: white;
					box-shadow: 0 2px 4px rgba(0, 0, 0, 0.2);
				}

				&:hover:not(.active) {
					color: #ccc;
				}
			}
		}
	}

	.input-group {
		display: flex;
		gap: 8px;

		input {
			flex: 1;
			background: rgba(0, 0, 0, 0.3);
			border: 1px solid rgba(255, 255, 255, 0.1);
			border-radius: 8px;
			padding: 10px 14px;
			color: #ccc;
			font-size: 14px;
			font-family: 'JetBrains Mono', monospace;

			&:focus {
				outline: none;
				border-color: var(--primary, #ff4655);
			}
		}

		.actions {
			display: flex;
			gap: 4px;
		}

		.action-btn {
			background: rgba(255, 255, 255, 0.05);
			border: 1px solid rgba(255, 255, 255, 0.1);
			color: #ccc;
			width: 42px;
			border-radius: 8px;
			cursor: pointer;
			display: flex;
			align-items: center;
			justify-content: center;
			transition: all 0.2s;

			&:hover {
				background: rgba(255, 255, 255, 0.1);
				color: white;
			}

			&.copy {
				&:hover {
					color: #10b981;
					border-color: #10b981;
					background: rgba(16, 185, 129, 0.1);
				}
			}
			&.regen {
				&:hover {
					color: #f59e0b;
					border-color: #f59e0b;
					background: rgba(245, 158, 11, 0.1);
				}
			}
		}
	}

	.stats {
		display: flex;
		align-items: center;
		gap: 6px;
		font-size: 13px;
		color: #ef4444; /* Warning color for 'Once' limits */
		padding-left: 4px;
	}

	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
	}
</style>
