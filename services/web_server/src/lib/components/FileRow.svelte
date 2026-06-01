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
				color={starred ? '#eab308' : 'inherit'}
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
		padding: 12px 16px;
		border-radius: var(--radius-md);
		background-color: var(--bg-card);
		border-bottom: 1px solid var(--border-default);
		transition: background-color 0.2s;
		cursor: pointer;

		&:hover {
			background-color: var(--bg-card-hover);
		}

		.file-main {
			display: flex;
			align-items: center;
			gap: 12px;

			.file-icon {
				width: 36px;
				height: 36px;
				border-radius: 8px;
				display: flex;
				align-items: center;
				justify-content: center;
				font-size: 18px;

				&.image {
					background: rgba(74, 163, 226, 0.1);
					color: #4aa3e2;
				}
				&.video {
					background: rgba(255, 70, 85, 0.1);
					color: #ff4655;
				}
				&.doc {
					background: rgba(31, 122, 74, 0.1);
					color: #1f7a4a;
				}
				&.audio {
					background: rgba(245, 166, 35, 0.1);
					color: #f5a623;
				}
			}

			.file-name {
				font-weight: 500;
				color: var(--text-primary);
			}
		}

		.file-date,
		.file-size {
			font-size: 14px;
			color: var(--text-muted);
		}

		.file-actions {
			display: flex;
			gap: 4px;
			opacity: 0;
			transition: opacity 0.2s;

			button {
				background: transparent;
				border: none;
				color: var(--text-muted);
				padding: 8px;
				border-radius: 50%;
				cursor: pointer;

				&:hover {
					color: var(--text-primary);
					background: var(--nav-hover);
				}
			}
		}

		&:hover .file-actions {
			opacity: 1;
		}
	}
</style>
