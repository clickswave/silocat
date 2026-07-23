<script>
	import Input from './Input.svelte';
	import IconButton from './IconButton.svelte';
	import { toast } from 'svelte-sonner';

	let {
		value = $bindable(''),
		label = undefined,
		placeholder = '••••••••',
		icon = 'ri:lock-2-line',
		copyable = false,
		generatable = false,
		hint = undefined,
		error = undefined,
		...rest
	} = $props();

	let show = $state(false);

	function generate() {
		const chars = 'ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz23456789';
		const buf = new Uint32Array(20);
		crypto.getRandomValues(buf);
		value = Array.from(buf, (n) => chars[n % chars.length]).join('');
		show = true;
	}

	async function copy() {
		await navigator.clipboard.writeText(value);
		toast.success('Copied to clipboard');
	}
</script>

<Input bind:value type={show ? 'text' : 'password'} {label} {icon} {placeholder} {hint} {error} mono={show} {...rest}>
	{#if generatable}
		<IconButton size="sm" icon="ri:refresh-line" label="Generate password" onclick={generate} />
	{/if}
	{#if copyable && value}
		<IconButton size="sm" icon="ri:file-copy-line" label="Copy password" onclick={copy} />
	{/if}
	<IconButton
		size="sm"
		icon={show ? 'ri:eye-off-line' : 'ri:eye-line'}
		label={show ? 'Hide password' : 'Show password'}
		onclick={() => (show = !show)}
	/>
</Input>
