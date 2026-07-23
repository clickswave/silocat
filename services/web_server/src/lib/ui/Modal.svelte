<script>
	import { fade, fly } from 'svelte/transition';
	import Icon from '@iconify/svelte';
	import IconButton from './IconButton.svelte';

	let {
		open = false,
		title = undefined,
		icon = undefined,
		size = 'md', // sm | md | lg
		onclose = undefined,
		children,
		footer = undefined
	} = $props();

	function onkeydown(e) {
		if (e.key === 'Escape' && open) onclose?.();
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
	<div class="scrim" transition:fade={{ duration: 120 }} onclick={() => onclose?.()} aria-hidden="true"></div>
	<div class="holder" role="dialog" aria-modal="true" aria-label={title}>
		<div class="dialog {size}" transition:fly={{ y: 8, duration: 150 }}>
			{#if title}
				<header class="head">
					<div class="title-row">
						{#if icon}<Icon {icon} width={17} />{/if}
						<h2 class="title">{title}</h2>
					</div>
					<IconButton icon="ri:close-line" label="Close" onclick={() => onclose?.()} />
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
		padding: var(--space-4);
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
		max-height: min(84vh, 720px);

		&.sm {
			max-width: 380px;
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
		justify-content: space-between;
		gap: var(--space-3);
		padding: var(--space-4) var(--space-5);
		border-bottom: 1px solid var(--edge);
	}

	.title-row {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		color: var(--ink);
		min-width: 0;
	}

	.title {
		font-size: var(--fs-body);
		font-weight: var(--fw-semibold);
		margin: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.body {
		padding: var(--space-5);
		overflow-y: auto;
	}

	.foot {
		display: flex;
		justify-content: flex-end;
		gap: var(--space-2);
		padding: var(--space-4) var(--space-5);
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
