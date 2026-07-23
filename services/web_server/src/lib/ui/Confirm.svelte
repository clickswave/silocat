<script>
	import Modal from './Modal.svelte';
	import Button from './Button.svelte';

	let {
		open = false,
		title = 'Are you sure?',
		message = '',
		confirmLabel = 'Confirm',
		cancelLabel = 'Cancel',
		danger = false,
		loading = false,
		onconfirm = undefined,
		onclose = undefined
	} = $props();
</script>

<Modal {open} {title} size="sm" {onclose}>
	{#if message}
		<p class="message">{message}</p>
	{/if}

	{#snippet footer()}
		<Button variant="quiet" onclick={() => onclose?.()}>{cancelLabel}</Button>
		<Button variant={danger ? 'danger-solid' : 'solid'} {loading} onclick={() => onconfirm?.()}>
			{confirmLabel}
		</Button>
	{/snippet}
</Modal>

<style lang="scss">
	.message {
		color: var(--ink-mute);
		font-size: var(--fs-body);
		line-height: var(--lh-normal);
		margin: 0;
	}
</style>
