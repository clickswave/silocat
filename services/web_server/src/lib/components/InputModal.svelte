<script>
	/**
	 * The single-field prompt: New Folder, Rename, Decrypt.
	 *
	 * Built on `ui/Modal` so it shares one shell with every other overlay. The
	 * field is labelled rather than relying on a bare placeholder, because a
	 * placeholder disappears the moment you type and "what was this asking for?"
	 * is the classic result.
	 */
	import Modal from '$lib/ui/Modal.svelte';

	let {
		show = $bindable(false),
		title = '',
		icon = undefined,
		/** Field label. Falls back to the title so callers can stay terse. */
		label = '',
		placeholder = '',
		hint = '',
		initialValue = '',
		submitLabel = 'Save',
		cancelLabel = 'Cancel',
		/** Passwords and keys render in mono, like everywhere else. */
		mono = false,
		type = 'text',
		onconfirm = () => {},
		onclose = () => {}
	} = $props();

	let value = $state(initialValue);
	let inputEl = $state(null);

	// Focus and pre-select on open: renaming should not require a manual select-all.
	$effect(() => {
		if (show && inputEl) {
			inputEl.focus();
			inputEl.select();
		}
	});

	function close() {
		show = false;
		value = initialValue;
		onclose?.();
	}

	function submit() {
		const v = value.trim();
		if (!v) return;
		onconfirm?.(v);
		show = false;
		value = initialValue;
	}

	function onkeydown(e) {
		if (e.key === 'Enter') {
			e.preventDefault();
			submit();
		}
	}
</script>

<Modal open={show} {title} {icon} size="sm" onclose={close}>
	<div class="field">
		<label class="label" for="prompt-input">{label || title}</label>
		<input
			id="prompt-input"
			bind:this={inputEl}
			bind:value
			{type}
			{placeholder}
			class:mono
			{onkeydown}
			autocomplete="off"
			spellcheck="false"
		/>
		{#if hint}
			<span class="hint">{hint}</span>
		{/if}
	</div>

	{#snippet footer()}
		<button type="button" class="ghost" onclick={close}>{cancelLabel}</button>
		<button type="button" class="primary" disabled={!value.trim()} onclick={submit}>
			{submitLabel}
		</button>
	{/snippet}
</Modal>

<style lang="scss">
	.field {
		display: flex;
		flex-direction: column;
		gap: 0.375rem;
	}

	.label {
		font-size: var(--fs-xs);
		color: var(--ink-mute);
	}

	input {
		height: 36px;
		padding: 0 0.625rem;
		border-radius: var(--radius-sm);
		background: var(--surface);
		border: 1px solid var(--edge);
		color: var(--ink);
		font-family: var(--font-sans);
		font-size: 0.875rem;
		outline: none;
		transition:
			border-color var(--dur-fast) var(--ease),
			box-shadow var(--dur-fast) var(--ease);

		&::placeholder {
			color: var(--ink-faint);
		}
		&:focus {
			border-color: var(--accent);
			box-shadow: 0 0 0 3px var(--focus-ring);
		}
		&.mono {
			font-family: var(--font-mono);
		}
	}

	.hint {
		font-size: var(--fs-xs);
		color: var(--ink-faint);
	}

	.ghost,
	.primary {
		height: 34px;
		padding-inline: 0.875rem;
		border-radius: var(--radius-md);
		border: 1px solid transparent;
		font: inherit;
		font-size: var(--fs-sm);
		font-weight: var(--fw-medium);
		cursor: pointer;
		transition:
			background var(--dur-fast) var(--ease),
			color var(--dur-fast) var(--ease),
			filter var(--dur-fast) var(--ease);
	}

	.ghost {
		background: none;
		color: var(--ink-mute);

		&:hover {
			background: var(--tint-soft);
			color: var(--ink);
		}
	}

	.primary {
		background: var(--accent);
		color: #fff;

		&:hover:not(:disabled) {
			filter: brightness(1.08);
		}
		&:disabled {
			opacity: 0.5;
			cursor: not-allowed;
		}
	}
</style>
