<script>
	import Icon from '@iconify/svelte';

	let {
		icon,
		label, // required for a11y
		size = 'md', // sm | md
		active = false,
		danger = false,
		disabled = false,
		onclick = undefined,
		...rest
	} = $props();
</script>

<button
	type="button"
	class="icon-button {size}"
	class:active
	class:danger
	{disabled}
	aria-label={label}
	title={label}
	{onclick}
	{...rest}
>
	<Icon {icon} width={size === 'sm' ? 15 : 17} />
</button>

<style lang="scss">
	.icon-button {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		background: transparent;
		border: none;
		border-radius: var(--radius-sm);
		color: var(--ink-mute);
		cursor: pointer;
		transition:
			background var(--dur) var(--ease),
			color var(--dur) var(--ease);

		&.sm {
			width: 26px;
			height: 26px;
		}
		&.md {
			width: 32px;
			height: 32px;
		}

		&:hover:not(:disabled) {
			background: var(--tint-soft);
			color: var(--ink);
		}
		&.active {
			background: var(--accent-soft);
			color: var(--accent);
		}
		&.danger:hover:not(:disabled) {
			background: var(--danger-soft);
			color: var(--danger);
		}
		&:disabled {
			opacity: 0.4;
			cursor: not-allowed;
		}
	}
</style>
