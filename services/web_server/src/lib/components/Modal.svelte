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
		background: rgba(0, 0, 0, 0.6);
		backdrop-filter: blur(8px);
		display: flex;
		justify-content: center;
		align-items: center;
		z-index: 1000;
		animation: fadeIn 0.2s ease-out;
	}

	.modal-content {
		background: rgba(22, 27, 34, 0.8);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: 16px;
		width: 90%;
		max-width: 450px;
		padding: 24px;
		box-shadow: 0 20px 40px rgba(0, 0, 0, 0.4);
		animation: slideUp 0.3s cubic-bezier(0.16, 1, 0.3, 1);
		display: flex;
		flex-direction: column;
		gap: 20px;
	}

	.modal-header {
		display: flex;
		justify-content: space-between;
		align-items: center;

		h3 {
			font-size: 20px;
			font-weight: 600;
			color: #fff;
			margin: 0;
		}

		.close-btn {
			background: transparent;
			border: none;
			color: rgba(255, 255, 255, 0.5);
			cursor: pointer;
			padding: 4px;
			border-radius: 50%;
			transition: all 0.2s;

			&:hover {
				color: #fff;
				background: rgba(255, 255, 255, 0.1);
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
