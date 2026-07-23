<script>
	let {
		checked = $bindable(false),
		label = undefined,
		disabled = false,
		onchange = undefined
	} = $props();
</script>

<label class="switch" class:disabled>
	<input type="checkbox" bind:checked {disabled} {onchange} />
	<span class="track" aria-hidden="true"><span class="thumb"></span></span>
	{#if label}<span class="text">{label}</span>{/if}
</label>

<style lang="scss">
	.switch {
		display: inline-flex;
		align-items: center;
		gap: var(--space-3);
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

	.track {
		position: relative;
		width: 36px;
		height: 21px;
		background: var(--tint-softer);
		border: 1px solid var(--edge-strong);
		border-radius: var(--radius-full);
		transition:
			background var(--dur) var(--ease),
			border-color var(--dur) var(--ease);
		flex-shrink: 0;
	}

	.thumb {
		position: absolute;
		top: 2px;
		left: 2px;
		width: 15px;
		height: 15px;
		background: var(--ink-mute);
		border-radius: var(--radius-full);
		transition:
			transform var(--dur) var(--ease),
			background var(--dur) var(--ease);
	}

	input:checked + .track {
		background: var(--accent);
		border-color: var(--accent);

		.thumb {
			transform: translateX(15px);
			background: #fff;
		}
	}

	input:focus-visible + .track {
		outline: 2px solid var(--accent);
		outline-offset: 2px;
	}

	.text {
		font-size: var(--fs-body);
		color: var(--ink);
	}
</style>
