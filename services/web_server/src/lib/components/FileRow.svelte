<script>
	import Icon from '@iconify/svelte';

	let { name, size, date, type = 'file', starred = false, ondownload, ondelete, onstar } = $props();
</script>

<div class="file-row">
	<div class="file-main">
		<div class="file-icon {type}">
			{#if type === 'image'}<Icon icon="ri:image-fill" />
			{:else if type === 'video'}<Icon icon="ri:film-fill" />
			{:else if type === 'doc'}<Icon icon="ri:file-text-fill" />
			{:else if type === 'audio'}<Icon icon="ri:music-fill" />
			{:else}<Icon icon="ri:file-fill" />
			{/if}
		</div>
		<span class="file-name">{name}</span>
	</div>

	<span class="file-date">{date}</span>
	<span class="file-size">{size}</span>

	<div class="file-actions">
		<button onclick={onstar} aria-label="Star">
			<Icon
				icon={starred ? 'ri:star-fill' : 'ri:star-line'}
				color={starred ? 'var(--warning)' : 'inherit'}
			/>
		</button>
		<button onclick={ondownload} aria-label="Download"><Icon icon="ri:download-line" /></button>
		<button onclick={ondelete} class="delete-btn" aria-label="Delete"
			><Icon icon="ri:delete-bin-line" /></button
		>
	</div>
</div>

<style lang="scss">
	.file-row {
		display: grid;
		grid-template-columns: 2fr 1fr 1fr auto;
		align-items: center;
		padding: var(--space-3) var(--space-4);
		border-radius: var(--radius-md);
		background-color: var(--bg-card);
		border-bottom: 1px solid var(--border-default);
		transition: background-color var(--dur) var(--ease);
		cursor: pointer;

		&:hover {
			background-color: var(--bg-card-hover);
		}

		.file-main {
			display: flex;
			align-items: center;
			gap: var(--space-3);

			.file-icon {
				width: 36px;
				height: 36px;
				border-radius: var(--radius-sm);
				display: flex;
				align-items: center;
				justify-content: center;
				font-size: var(--fs-lg);
				color: var(--text-secondary);
				background: var(--tint-soft);

			}

			.file-name {
				font-weight: var(--fw-medium);
				color: var(--text-primary);
			}
		}

		.file-date,
		.file-size {
			font-size: var(--fs-sm);
			color: var(--text-muted);
		}

		.file-actions {
			display: flex;
			gap: var(--space-1);
			opacity: 0;
			transition: opacity var(--dur) var(--ease);

			button {
				background: transparent;
				border: none;
				color: var(--text-muted);
				padding: var(--space-2);
				border-radius: var(--radius-sm);
				cursor: pointer;
				display: flex;
				transition: color var(--dur) var(--ease), background var(--dur) var(--ease);

				&:hover {
					color: var(--text-primary);
					background: var(--nav-hover);
				}

				&.delete-btn:hover {
					color: var(--danger);
					background: var(--danger-soft);
				}
			}
		}

		&:hover .file-actions {
			opacity: 1;
		}
	}
</style>
