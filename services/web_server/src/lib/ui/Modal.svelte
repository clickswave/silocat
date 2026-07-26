<script>
	import { fade, scale } from 'svelte/transition';
	import { cubicOut } from 'svelte/easing';
	import Icon from './Icon.svelte';

	let {
		open = false,
		title = undefined,
		icon = undefined,
		/** Tint of the header icon chip: neutral | ok | warn | danger | accent */
		iconTone = 'neutral',
		size = 'md', // sm | md | lg
		/** Set false while an operation must not be interrupted (mid-upload). */
		dismissible = true,
		onclose = undefined,
		children,
		footer = undefined
	} = $props();

	function requestClose() {
		if (dismissible) onclose?.();
	}

	function onkeydown(e) {
		if (e.key === 'Escape' && open) requestClose();
	}

	$effect(() => {
		if (!open) return;
		document.body.style.overflow = 'hidden';
		return () => {
			document.body.style.overflow = '';
		};
	});
</script>

<svelte:window {onkeydown} />

{#if open}
	<div
		class="scrim"
		transition:fade={{ duration: 150 }}
		onclick={requestClose}
		aria-hidden="true"
	></div>
	<div class="holder" role="dialog" aria-modal="true" aria-label={title}>
		<div
			class="dialog {size}"
			transition:scale={{ duration: 190, start: 0.96, easing: cubicOut }}
		>
			{#if title}
				<header class="head">
					{#if icon}
						<span class="chip {iconTone}"><Icon name={icon} size={16} /></span>
					{/if}
					<span class="title">{title}</span>
					<button type="button" class="close" aria-label="Close" onclick={requestClose}>
						<Icon name="close" size={15} />
					</button>
				</header>
			{/if}
			<div class="body">
				{@render children?.()}
			</div>
			{#if footer}
				<footer class="foot">
					{@render footer()}
				</footer>
			{/if}
		</div>
	</div>
{/if}

<style lang="scss">
	.scrim {
		position: fixed;
		inset: 0;
		background: var(--scrim);
		z-index: 1000;
	}

	.holder {
		position: fixed;
		inset: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: var(--space-5);
		z-index: 1001;
		pointer-events: none;
	}

	.dialog {
		width: 100%;
		background: var(--raised);
		border: 1px solid var(--edge);
		border-radius: var(--radius-lg);
		box-shadow: var(--shadow-overlay);
		pointer-events: auto;
		display: flex;
		flex-direction: column;
		overflow: hidden;
		max-height: min(84vh, 760px);

		&.sm {
			max-width: 400px;
		}
		&.md {
			max-width: 460px;
		}
		&.lg {
			max-width: 620px;
		}
	}

	.head {
		display: flex;
		align-items: center;
		gap: 0.625rem;
		padding: 1rem 1rem 0.875rem;
	}

	.chip {
		display: grid;
		place-items: center;
		width: 28px;
		height: 28px;
		border-radius: 8px;
		flex: 0 0 auto;

		&.neutral {
			background: var(--tint-soft);
			color: var(--ink-mute);
		}
		&.ok {
			background: var(--ok-soft);
			color: var(--ok);
		}
		&.warn {
			background: var(--warn-soft);
			color: var(--warn);
		}
		&.danger {
			background: var(--danger-soft);
			color: var(--danger);
		}
		&.accent {
			background: var(--accent-soft);
			color: var(--accent);
		}
	}

	.title {
		flex: 1;
		min-width: 0;
		font-size: 0.9375rem;
		font-weight: var(--fw-semibold);
		letter-spacing: var(--tracking-tight);
		color: var(--ink);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.close {
		flex: 0 0 auto;
		width: 28px;
		height: 28px;
		border: 0;
		background: none;
		border-radius: var(--radius-sm);
		display: grid;
		place-items: center;
		color: var(--ink-faint);
		cursor: pointer;
		transition:
			background var(--dur-fast) var(--ease),
			color var(--dur-fast) var(--ease);

		&:hover {
			background: var(--tint-soft);
			color: var(--ink);
		}
	}

	.body {
		padding: 0 1rem 1rem;
		overflow-y: auto;
	}

	.foot {
		display: flex;
		align-items: center;
		justify-content: flex-end;
		gap: var(--space-2);
		padding: 0.875rem 1rem;
		border-top: 1px solid var(--edge);
	}

	/* bottom sheet on small screens */
	@media (max-width: 640px) {
		.holder {
			align-items: flex-end;
			padding: 0;
		}
		.dialog {
			max-width: none !important;
			border-radius: var(--radius-lg) var(--radius-lg) 0 0;
			border-bottom: none;
			max-height: 88vh;
		}
	}
</style>
