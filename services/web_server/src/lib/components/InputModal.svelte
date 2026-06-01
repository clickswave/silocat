<script>
	import Modal from './Modal.svelte';

	export let show = false; // Add show prop for bind:show
	export let title = 'Enter Value';
	export let placeholder = '';
	export let initialValue = ''; // Renamed from value to match +page.svelte
	export let submitLabel = 'Confirm';
	export let cancelLabel = 'Cancel';
	export let icon = ''; // Add icon prop since it's used in +page.svelte

	// Callback props
	export let onsubmit = (val) => {};
	export let onconfirm = (val) => {}; // Alias for onsubmit if used interchangeably
	export let onclose = () => {};

	let value = initialValue;
	let inputEl;

	function handleSubmit() {
		if (value.trim()) {
			if (onsubmit) onsubmit(value);
			if (onconfirm) onconfirm(value);
			show = false; // Close on success
		}
	}

	function handleClose() {
		show = false;
		if (onclose) onclose();
	}

	function handleKeyDown(e) {
		if (e.key === 'Enter') {
			handleSubmit();
		}
	}
</script>

{#if show}
	<Modal {title} {icon} onclose={handleClose}>
		<div class="input-container">
			<input
				type="text"
				bind:value
				bind:this={inputEl}
				{placeholder}
				onkeydown={handleKeyDown}
				autofocus
			/>
		</div>
		<div class="actions">
			<button class="btn-cancel" onclick={handleClose}>{cancelLabel}</button>
			<button class="btn-submit" onclick={handleSubmit} disabled={!value.trim()}
				>{submitLabel}</button
			>
		</div>
	</Modal>
{/if}

<style lang="scss">
	.input-container {
		width: 100%;

		input {
			width: 100%;
			background: rgba(255, 255, 255, 0.05);
			border: 1px solid rgba(255, 255, 255, 0.1);
			border-radius: 8px;
			padding: 12px 16px;
			color: #fff;
			font-size: 16px;
			outline: none;
			transition: all 0.2s;

			&:focus {
				border-color: #3b82f6;
				background: rgba(59, 130, 246, 0.1);
			}

			&::placeholder {
				color: rgba(255, 255, 255, 0.3);
			}
		}
	}

	.actions {
		display: flex;
		justify-content: flex-end;
		gap: 12px;
		margin-top: 8px;

		button {
			padding: 10px 20px;
			border-radius: 8px;
			font-size: 14px;
			font-weight: 500;
			cursor: pointer;
			transition: all 0.2s;
			border: none;
		}

		.btn-cancel {
			background: transparent;
			color: rgba(255, 255, 255, 0.6);
			border: 1px solid rgba(255, 255, 255, 0.1);

			&:hover {
				background: rgba(255, 255, 255, 0.05);
				color: #fff;
			}
		}

		.btn-submit {
			background: #3b82f6;
			color: #fff;

			&:hover {
				background: #2563eb;
			}

			&:disabled {
				opacity: 0.5;
				cursor: not-allowed;
			}
		}
	}
</style>
