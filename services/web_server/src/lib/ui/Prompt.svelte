<script>
	import Modal from './Modal.svelte';
	import Button from './Button.svelte';
	import Input from './Input.svelte';
	import PasswordInput from './PasswordInput.svelte';

	let {
		open = false,
		title = '',
		message = undefined,
		placeholder = '',
		initial = '',
		type = 'text', // text | password
		submitLabel = 'Save',
		cancelLabel = 'Cancel',
		loading = false,
		onsubmit = undefined,
		onclose = undefined
	} = $props();

	let value = $state('');
	let inputEl = $state();

	$effect(() => {
		if (open) {
			value = initial;
			setTimeout(() => inputEl?.querySelector('input')?.focus(), 30);
		}
	});

	function submit() {
		if (!value.trim()) return;
		onsubmit?.(value.trim());
	}

	function onkeydown(e) {
		if (e.key === 'Enter' && open) submit();
	}
</script>

<svelte:window {onkeydown} />

<Modal {open} {title} size="sm" {onclose}>
	<div class="stack" bind:this={inputEl}>
		{#if message}
			<p class="message">{message}</p>
		{/if}
		{#if type === 'password'}
			<PasswordInput bind:value {placeholder} />
		{:else}
			<Input bind:value {placeholder} />
		{/if}
	</div>

	{#snippet footer()}
		<Button variant="quiet" onclick={() => onclose?.()}>{cancelLabel}</Button>
		<Button variant="solid" {loading} disabled={!value.trim()} onclick={submit}>{submitLabel}</Button>
	{/snippet}
</Modal>

<style lang="scss">
	.stack {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
	}
	.message {
		color: var(--ink-mute);
		font-size: var(--fs-sm);
		margin: 0;
	}
</style>
