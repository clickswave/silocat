<script>
	import { createEventDispatcher } from 'svelte';
	import Icon from '@iconify/svelte';

	const dispatch = createEventDispatcher();

	export let title = '';
	export let icon = ''; // Add icon prop
	export let onclose = () => {}; // Add onclose callback prop
	export let hideHeader = false;

	function close() {
		if (onclose) onclose();
		dispatch('close');
	}
</script>

<div class="modal-backdrop" on:click={close}>
	<div class="modal-content" on:click|stopPropagation>
		{#if !hideHeader}
			<div class="modal-header">
				<h3>{title}</h3>
				<button class="close-btn" on:click={close}>
					<Icon icon="ri:close-line" width="24" />
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
		max-width: 450px;
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

		h3 {
			font-size: var(--fs-h3);
			font-weight: var(--fw-semibold);
			color: var(--text-primary);
			margin: 0;
		}

		.close-btn {
			background: transparent;
			border: none;
			color: var(--text-muted);
			cursor: pointer;
			padding: var(--space-1);
			border-radius: var(--radius-sm);
			display: flex;
			transition: color var(--dur) var(--ease), background var(--dur) var(--ease);

			&:hover {
				color: var(--text-primary);
				background: var(--tint-softer);
			}
		}
	}

	.modal-body {
		/* Add slot styles if needed */
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
</style>
