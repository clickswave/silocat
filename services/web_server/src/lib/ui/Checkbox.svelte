<script>
	import Icon from '@iconify/svelte';

	let {
		checked = $bindable(false),
		label = undefined,
		disabled = false,
		onchange = undefined
	} = $props();
</script>

<label class="checkbox" class:disabled>
	<input type="checkbox" bind:checked {disabled} {onchange} />
	<span class="box" aria-hidden="true">
		{#if checked}<Icon icon="ri:check-line" width={13} />{/if}
	</span>
	{#if label}<span class="text">{label}</span>{/if}
</label>

<style lang="scss">
	.checkbox {
		display: inline-flex;
		align-items: center;
		gap: var(--space-2);
		cursor: pointer;
		user-select: none;

		&.disabled {
			opacity: 0.5;
			cursor: not-allowed;
		}

		input {
			position: absolute;
			opacity: 0;
			width: 0;
			height: 0;
		}
	}

	.box {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 17px;
		height: 17px;
		background: var(--bg);
		border: 1px solid var(--edge-strong);
		border-radius: 4px;
		color: #fff;
		transition:
			background var(--dur) var(--ease),
			border-color var(--dur) var(--ease);
		flex-shrink: 0;
	}

	input:checked + .box {
		background: var(--accent);
		border-color: var(--accent);
	}

	input:focus-visible + .box {
		outline: 2px solid var(--accent);
		outline-offset: 2px;
	}

	.text {
		font-size: var(--fs-sm);
		color: var(--ink);
	}
</style>
