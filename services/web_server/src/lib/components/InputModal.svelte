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
			background: var(--bg-input);
			border: 1px solid var(--border-default);
			border-radius: var(--radius-sm);
			padding: 0.75rem 0.95rem;
			color: var(--text-primary);
			font-family: inherit;
			font-size: var(--fs-body);
			outline: none;
			transition: border-color var(--dur) var(--ease), box-shadow var(--dur) var(--ease);

			&:focus {
				border-color: var(--primary);
				box-shadow: 0 0 0 3px var(--primary-glow);
			}

			&::placeholder {
				color: var(--text-muted);
			}
		}
	}

	.actions {
		display: flex;
		justify-content: flex-end;
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
				filter var(--dur) var(--ease), box-shadow var(--dur) var(--ease);
			border: 1px solid transparent;
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

		.btn-submit {
			background: var(--accent-gradient);
			color: #fff;
			box-shadow: 0 6px 20px -6px var(--primary-glow);

			&:hover {
				filter: brightness(1.06);
				box-shadow: 0 10px 28px -6px var(--primary-glow);
			}

			&:disabled {
				opacity: 0.55;
				cursor: not-allowed;
			}
		}
	}
</style>
