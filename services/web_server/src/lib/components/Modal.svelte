<script>
	import { createEventDispatcher } from 'svelte';
	import Icon from '@iconify/svelte';

	const dispatch = createEventDispatcher();

	export let title = '';
	export let icon = '';
	export let onclose = () => {};
	export let hideHeader = false;

	function close() {
		if (onclose) onclose();
		dispatch('close');
	}
</script>

<div class="modal-backdrop" on:click={close} role="presentation">
	<div class="modal-content" on:click|stopPropagation>
		{#if !hideHeader}
			<div class="modal-header">
				<div class="modal-title">
					{#if icon}
						<span class="title-icon"><Icon {icon} width="20" /></span>
					{/if}
					<h3>{title}</h3>
				</div>
				<button class="close-btn" on:click={close} aria-label="Close">
					<Icon icon="ri:close-line" width="22" />
				</button>
			</div>
		{/if}
		<div class="modal-body">
			<slot />
		</div>
	</div>
</div>

<style lang="scss">
	.modal-backdrop {
		position: fixed;
		top: 0;
		left: 0;
		width: 100%;
		height: 100vh;
		background: rgba(0, 0, 0, 0.65);
		backdrop-filter: blur(8px);
		display: flex;
		justify-content: center;
		align-items: center;
		z-index: 1000;
		padding: var(--gutter);
		animation: fadeIn 0.2s ease-out;
	}

	.modal-content {
		background: var(--bg-elevated);
		border: 1px solid var(--border-default);
		border-radius: var(--radius-lg);
		width: 100%;
		max-width: 440px;
		max-height: 90vh;
		overflow-y: auto;
		padding: var(--space-6);
		box-shadow: var(--shadow-lg);
		animation: slideUp 0.3s cubic-bezier(0.16, 1, 0.3, 1);
		display: flex;
		flex-direction: column;
		gap: var(--space-5);
	}

	.modal-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		gap: var(--space-3);

		.modal-title {
			display: flex;
			align-items: center;
			gap: var(--space-3);
			min-width: 0;

			.title-icon {
				display: grid;
				place-items: center;
				width: 34px;
				height: 34px;
				flex-shrink: 0;
				border-radius: var(--radius-md);
				background: var(--tint-soft);
				color: var(--primary);
			}

			h3 {
				font-size: var(--fs-lg);
				font-weight: var(--fw-semibold);
				color: var(--text-primary);
				margin: 0;
				overflow: hidden;
				text-overflow: ellipsis;
				white-space: nowrap;
			}
		}

		.close-btn {
			background: transparent;
			border: none;
			color: var(--text-muted);
			cursor: pointer;
			padding: 6px;
			margin: -6px -4px -6px 0;
			border-radius: var(--radius-sm);
			display: flex;
			flex-shrink: 0;
			transition: color var(--dur) var(--ease), background var(--dur) var(--ease);

			&:hover {
				color: var(--text-primary);
				background: var(--tint-soft);
			}
		}
	}

	.modal-body {
		display: flex;
		flex-direction: column;
		gap: var(--space-5);
	}

	@keyframes fadeIn {
		from {
			opacity: 0;
		}
		to {
			opacity: 1;
		}
	}

	@keyframes slideUp {
		from {
			transform: translateY(20px);
			opacity: 0;
		}
		to {
			transform: translateY(0);
			opacity: 1;
		}
	}

	/* Bottom-sheet on small screens */
	@media (max-width: 600px) {
		.modal-backdrop {
			align-items: flex-end;
			padding: 0;
		}
		.modal-content {
			max-width: 100%;
			max-height: 92vh;
			border-radius: var(--radius-lg) var(--radius-lg) 0 0;
			animation: sheetUp 0.28s cubic-bezier(0.16, 1, 0.3, 1);
		}
	}
	@keyframes sheetUp {
		from {
			transform: translateY(100%);
		}
		to {
			transform: translateY(0);
		}
	}
</style>
