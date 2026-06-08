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
		gap: var(--space-5);
		padding: var(--space-2);
	}

	.icon-wrapper {
		width: 64px;
		height: 64px;
		border-radius: var(--radius-pill);
		background: var(--tint-softer);
		color: var(--text-secondary);
		display: flex;
		align-items: center;
		justify-content: center;
		margin-bottom: calc(-1 * var(--space-2));

		&.danger {
			background: rgba(255, 70, 85, 0.1);
			color: var(--danger);
		}
	}

	.text-content {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);

		h3 {
			font-size: var(--fs-h3);
			font-weight: var(--fw-semibold);
			color: var(--text-primary);
			margin: 0;
		}

		.message {
			color: var(--text-secondary);
			font-size: var(--fs-sm);
			line-height: var(--lh-normal);
			margin: 0;
			max-width: 320px;
		}
	}

	.actions {
		display: flex;
		width: 100%;
		justify-content: center;
		gap: var(--space-3);
		margin-top: var(--space-2);

		button {
			padding: 0.7rem 1.25rem;
			border-radius: var(--radius-pill);
			font-family: inherit;
			font-size: var(--fs-body);
			font-weight: var(--fw-semibold);
			cursor: pointer;
			transition: background var(--dur) var(--ease), border-color var(--dur) var(--ease),
				filter var(--dur) var(--ease), transform var(--dur) var(--ease), box-shadow var(--dur) var(--ease);
			border: 1px solid transparent;
			min-width: 110px;
		}

		.btn-cancel {
			background: var(--tint-soft);
			color: var(--text-primary);
			border-color: var(--border-default);

			&:hover {
				background: var(--tint-softer);
				border-color: var(--border-strong);
			}
		}

		.btn-confirm {
			background: var(--accent-gradient);
			color: #fff;
			box-shadow: 0 6px 20px -6px var(--primary-glow);

			&:hover {
				filter: brightness(1.06);
				box-shadow: 0 10px 28px -6px var(--primary-glow);
			}

			&.danger {
				background: var(--accent-gradient);

				&:hover {
					filter: brightness(1.06);
				}
			}
		}
	}
</style>
