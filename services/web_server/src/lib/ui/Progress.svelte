<script>
	let {
		value = 0, // 0..100
		tone = 'accent', // accent | ok | warn | danger | neutral
		size = 'sm', // xs | sm | md
		indeterminate = false,
		label = undefined // a11y label
	} = $props();

	let clamped = $derived(Math.max(0, Math.min(100, value)));
</script>

<div
	class="progress {size}"
	role="progressbar"
	aria-label={label}
	aria-valuenow={indeterminate ? undefined : Math.round(clamped)}
	aria-valuemin="0"
	aria-valuemax="100"
>
	<div class="fill {tone}" class:indeterminate style="width: {indeterminate ? 40 : clamped}%"></div>
</div>

<style lang="scss">
	.progress {
		width: 100%;
		background: var(--tint-softer);
		border-radius: var(--radius-full);
		overflow: hidden;

		&.xs {
			height: 3px;
		}
		&.sm {
			height: 5px;
		}
		&.md {
			height: 8px;
		}
	}

	.fill {
		height: 100%;
		border-radius: var(--radius-full);
		transition: width 0.25s var(--ease);

		&.accent {
			background: var(--accent);
		}
		&.ok {
			background: var(--ok);
		}
		&.warn {
			background: var(--warn);
		}
		&.danger {
			background: var(--danger);
		}
		&.neutral {
			background: var(--ink-faint);
		}

		&.indeterminate {
			animation: slide 1.1s var(--ease) infinite;
		}
	}

	@keyframes slide {
		from {
			transform: translateX(-100%);
		}
		to {
			transform: translateX(350%);
		}
	}
</style>
