<script>
	import Icon from './Icon.svelte';

	let { text = '', label = 'Copy', size = 'md' } = $props();

	let copied = $state(false);
	let timer;

	async function copy() {
		await navigator.clipboard.writeText(text);
		copied = true;
		clearTimeout(timer);
		timer = setTimeout(() => (copied = false), 1600);
	}
</script>

<button
	type="button"
	class="copy {size}"
	class:copied
	aria-label={label}
	title={label}
	onclick={copy}
>
	<Icon icon={copied ? 'ri:check-line' : 'ri:file-copy-line'} width={size === 'sm' ? 14 : 16} />
</button>

<style lang="scss">
	.copy {
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

		&:hover {
			background: var(--tint-soft);
			color: var(--ink);
		}
		&.copied {
			color: var(--ok);
		}
	}
</style>
