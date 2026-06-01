<script>
	import Modal from './Modal.svelte';
	import Icon from '@iconify/svelte';

	export let show = false; // Add show prop for bind:show
	export let title = 'Are you sure?';
	export let message = 'This action cannot be undone.';
	export let confirmLabel = 'Delete';
	export let cancelLabel = 'Cancel';
	export let danger = false;
	export let isDanger = false; // Add alias to match +page.svelte usage if any

	// Callback props
	export let onconfirm = () => {};
	export let onclose = () => {};

	$: isDangerMode = danger || isDanger;

	function handleConfirm() {
		if (onconfirm) onconfirm();
		// show = false; // Let parent control visibility or onconfirm handle it?
		// Logic in parent usually sets show=false, but let's ensure we close or respect the flow.
		// Actually, ConfirmModal usually closes itself on action unless async?
		// Parent implementation: confirmDeleteFolder() sets showDeleteFolderModal = false.
		// So we don't strictly need to set show=false here if parent does.
		// But for safety, we can. However, if onconfirm is async and we want to show loading, we shouldn't close immediately.
		// Existing code: show = false inside handleConfirm. Let's keep it consistent.
		// Wait, original code had:
		/*
        function handleConfirm() {
            if (onconfirm) onconfirm();
            show = false;
        }
        */
		// I will keep it.
		show = false;
	}

	function handleClose() {
		show = false;
		if (onclose) onclose();
	}
</script>

{#if show}
	<Modal {title} onclose={handleClose} hideHeader={true}>
		<div class="confirm-content">
			<div class="icon-wrapper {isDangerMode ? 'danger' : ''}">
				<Icon icon="ri:delete-bin-5-fill" width="32" />
			</div>

			<div class="text-content">
				<h3>{title}</h3>
				<p class="message">{message}</p>
			</div>

			<div class="actions">
				<button class="btn-cancel" onclick={handleClose}>{cancelLabel}</button>
				<button class="btn-confirm {isDangerMode ? 'danger' : ''}" onclick={handleConfirm}
					>{confirmLabel}</button
				>
			</div>
		</div>
	</Modal>
{/if}

<style lang="scss">
	.confirm-content {
		display: flex;
		flex-direction: column;
		align-items: center;
		text-align: center;
		gap: 24px;
		padding: 8px;
	}

	.icon-wrapper {
		width: 64px;
		height: 64px;
		border-radius: 50%;
		background: rgba(59, 130, 246, 0.1);
		color: #3b82f6;
		display: flex;
		align-items: center;
		justify-content: center;
		margin-bottom: -8px;

		&.danger {
			background: rgba(239, 68, 68, 0.1);
			color: #ef4444;
		}
	}

	.text-content {
		display: flex;
		flex-direction: column;
		gap: 8px;

		h3 {
			font-size: 20px;
			font-weight: 600;
			color: #fff;
			margin: 0;
		}

		.message {
			color: var(--text-muted, rgba(255, 255, 255, 0.6));
			font-size: 15px;
			line-height: 1.5;
			margin: 0;
			max-width: 300px;
		}
	}

	.actions {
		display: flex;
		width: 100%;
		justify-content: center;
		gap: 12px;
		margin-top: 8px;

		button {
			padding: 10px 24px;
			border-radius: 10px;
			font-size: 14px;
			font-weight: 500;
			cursor: pointer;
			transition: all 0.2s;
			border: none;
			min-width: 100px;
		}

		.btn-cancel {
			background: transparent;
			color: rgba(255, 255, 255, 0.7);
			border: 1px solid rgba(255, 255, 255, 0.1);

			&:hover {
				background: rgba(255, 255, 255, 0.05);
				color: #fff;
				border-color: rgba(255, 255, 255, 0.2);
			}
		}

		.btn-confirm {
			background: #3b82f6;
			color: #fff;
			box-shadow: 0 4px 12px rgba(59, 130, 246, 0.3);

			&:hover {
				background: #2563eb;
				transform: translateY(-1px);
			}

			&.danger {
				background: #ef4444;
				box-shadow: 0 4px 12px rgba(239, 68, 68, 0.3);

				&:hover {
					background: #dc2626;
				}
			}
		}
	}
</style>
