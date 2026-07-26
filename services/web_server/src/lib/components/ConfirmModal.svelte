<script>
	import { fade, scale } from 'svelte/transition';
	import { cubicOut } from 'svelte/easing';
	import Icon from '$lib/ui/Icon.svelte';

	let {
		show = $bindable(false),
		title = 'Are you sure?',
		message = 'This action cannot be undone.',
		confirmLabel = 'Delete',
		cancelLabel = 'Cancel',
		/** `isDanger` kept as an alias: both spellings are in use across pages. */
		danger = false,
		isDanger = false,
		icon = 'trash',
		busy = false,
		onconfirm = () => {},
		onclose = () => {}
	} = $props();

	let dangerMode = $derived(danger || isDanger);

	function close() {
		show = false;
		onclose?.();
	}

	function confirm() {
		onconfirm?.();
	}

	function onkeydown(e) {
		if (e.key === 'Escape' && show) close();
	}
</script>

<svelte:window {onkeydown} />

{#if show}
	<div
		class="scrim"
		transition:fade={{ duration: 150 }}
		onclick={close}
		role="presentation"
	></div>
	<div class="holder" role="alertdialog" aria-modal="true" aria-label={title}>
		<div class="dialog" transition:scale={{ duration: 190, start: 0.96, easing: cubicOut }}>
			<div class="body">
				<span class="glyph" class:danger={dangerMode}>
					<Icon name={icon} size={20} />
				</span>
				<span class="title">{title}</span>
				<span class="message">{message}</span>
			</div>
			<div class="foot">
				<button type="button" class="cancel" onclick={close}>{cancelLabel}</button>
				<button
					type="button"
					class="confirm"
					class:danger={dangerMode}
					disabled={busy}
					onclick={confirm}
				>
					{confirmLabel}
				</button>
			</div>
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
		max-width: 400px;
		display: flex;
		flex-direction: column;
		background: var(--raised);
		border: 1px solid var(--edge);
		border-radius: var(--radius-lg);
		box-shadow: var(--shadow-overlay);
		overflow: hidden;
		pointer-events: auto;
	}

	.body {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.875rem;
		padding: 1.5rem 1.25rem 1rem;
		text-align: center;
	}

	.glyph {
		display: grid;
		place-items: center;
		width: 44px;
		height: 44px;
		border-radius: var(--radius-full);
		background: var(--tint-softer);
		color: var(--ink-mute);

		&.danger {
			background: var(--danger-soft);
			color: var(--danger);
		}
	}

	.title {
		font-size: var(--fs-lg);
		font-weight: var(--fw-semibold);
		letter-spacing: var(--tracking-tight);
	}

	.message {
		font-size: 0.875rem;
		color: var(--ink-mute);
		line-height: var(--lh-normal);
		max-width: 40ch;
	}

	.foot {
		display: flex;
		justify-content: flex-end;
		gap: var(--space-2);
		padding: 0.875rem 1rem;
		border-top: 1px solid var(--edge);
	}

	.cancel,
	.confirm {
		height: 34px;
		padding-inline: 0.875rem;
		border-radius: var(--radius-md);
		border: 0;
		font: inherit;
		font-size: var(--fs-sm);
		cursor: pointer;
		transition:
			background var(--dur-fast) var(--ease),
			color var(--dur-fast) var(--ease),
			filter var(--dur-fast) var(--ease);
	}

	.cancel {
		background: none;
		color: var(--ink-mute);

		&:hover {
			background: var(--tint-soft);
			color: var(--ink);
		}
	}

	.confirm {
		background: var(--accent);
		color: #fff;
		font-weight: var(--fw-medium);

		&.danger {
			background: var(--danger);
		}
		&:hover:not(:disabled) {
			filter: brightness(1.08);
		}
		&:disabled {
			opacity: 0.6;
			cursor: not-allowed;
		}
	}
</style>
