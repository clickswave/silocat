<script>
	import Icon from '@iconify/svelte';

	export let name = 'File';
	export let size = '0 B';
	export let date = '';
	export let type = 'file'; // image, video, audio, doc, file
	export let encrypted = false;
	export let starred = false;

	// Callbacks
	export let onclick = () => {};
	export let ondownload = () => {};
	export let ondelete = () => {};
	export let onstar = () => {};
	export let onshare = () => {};
	export let onrestore = () => {};

	export let isTrash = false;

	let showMenu = false;
	let menuRef;

	function toggleMenu(e) {
		e.stopPropagation();
		showMenu = !showMenu;
	}

	function closeMenu() {
		showMenu = false;
	}

	function handleDownload(e) {
		e.stopPropagation();
		closeMenu();
		if (ondownload) ondownload();
	}

	function handleStar(e) {
		e.stopPropagation();
		if (onstar) onstar();
	}

	function handleShare(e) {
		e.stopPropagation();
		console.log('FileCard: Share clicked');
		closeMenu();
		if (onshare) onshare();
	}

	function handleRestore(e) {
		e.stopPropagation();
		closeMenu();
		if (onrestore) onrestore();
	}

	function handleDelete(e) {
		e.stopPropagation();
		closeMenu();
		if (ondelete) ondelete();
	}

	function handleClickOutside(event) {
		if (showMenu && menuRef && !menuRef.contains(event.target)) {
			closeMenu();
		}
	}
</script>

<svelte:window on:click={handleClickOutside} />

<div
	class="file-card"
	on:click={onclick}
	role="button"
	tabindex="0"
	on:keydown={(e) => e.key === 'Enter' && onclick()}
>
	<div class="file-content">
		<div class="icon-section">
			<div class="file-icon {type}">
				{#if type === 'image'}<Icon icon="ri:image-2-fill" width="32" />
				{:else if type === 'video'}<Icon icon="ri:film-fill" width="32" />
				{:else if type === 'doc'}<Icon icon="ri:file-text-fill" width="32" />
				{:else if type === 'audio'}<Icon icon="ri:music-fill" width="32" />
				{:else}<Icon icon="ri:file-fill" width="32" />
				{/if}
			</div>
			<div class="indicators">
				{#if starred}
					<div class="star-indicator" title="Starred">
						<Icon icon="ri:star-fill" width="14" />
					</div>
				{/if}
				{#if encrypted}
					<div class="lock-indicator" title="Encrypted">
						<Icon icon="ri:lock-fill" width="14" />
					</div>
				{/if}
			</div>
		</div>

		<div class="info-section">
			<span class="name" title={name}>{name}</span>
			<div class="meta">
				<span class="size">{size}</span>
			</div>
		</div>
	</div>

	<div class="menu-container {showMenu ? 'visible' : ''}" bind:this={menuRef}>
		<button class="menu-btn" on:click={toggleMenu}>
			<Icon icon="ri:more-2-fill" width="20" />
		</button>
		{#if showMenu}
			<div class="dropdown-menu">
				{#if isTrash}
					<button class="dropdown-item" on:click={handleRestore}>
						<Icon icon="ri:arrow-go-back-line" width="16" />
						Restore
					</button>
					<button class="dropdown-item danger" on:click={handleDelete}>
						<Icon icon="ri:delete-bin-line" width="16" />
						Delete Forever
					</button>
				{:else}
					<button class="dropdown-item" on:click={handleDownload}>
						<Icon icon="ri:download-line" width="16" />
						Download
					</button>
					<button class="dropdown-item" on:click={handleStar}>
						<Icon
							icon={starred ? 'ri:star-fill' : 'ri:star-line'}
							width="16"
							color={starred ? 'var(--warning)' : 'inherit'}
						/>
						{starred ? 'Unstar' : 'Star'}
					</button>
					<button class="dropdown-item" on:click={handleShare}>
						<Icon icon="ri:share-forward-line" width="16" />
						Share
					</button>
					<button class="dropdown-item danger" on:click={handleDelete}>
						<Icon icon="ri:delete-bin-line" width="16" />
						Delete
					</button>
				{/if}
			</div>
		{/if}
	</div>
</div>

<style lang="scss">
	.file-card {
		background: var(--bg-card);
		border: 1px solid var(--border-default);
		border-radius: var(--radius-md);
		padding: var(--space-4);
		display: flex;
		flex-direction: column;
		justify-content: space-between;
		position: relative;
		cursor: pointer;
		min-height: 160px;
		box-shadow: var(--shadow-card);
		transition: background var(--dur) var(--ease), border-color var(--dur) var(--ease),
			box-shadow var(--dur) var(--ease);

		&:hover {
			background: var(--bg-card-hover);
			border-color: var(--border-active);
			box-shadow: var(--shadow-card);

			.menu-container .menu-btn {
				opacity: 1;
			}
		}

		.file-content {
			display: flex;
			flex-direction: column;
			flex: 1;
			gap: var(--space-4);
		}

		.icon-section {
			display: flex;
			align-items: flex-start;
			justify-content: flex-start;
			gap: var(--space-3);

			.file-icon {
				color: var(--text-secondary);
				transition: color var(--dur) var(--ease);
			}

			.indicators {
				display: flex;
				gap: var(--space-1);
				align-items: center;

				.lock-indicator {
					color: var(--warning);
					opacity: 0.85;
				}
				.star-indicator {
					color: var(--warning);
					opacity: 0.85;
				}
			}
		}

		.info-section {
			margin-top: auto;
			display: flex;
			flex-direction: column;
			gap: var(--space-1);

			.name {
				font-size: var(--fs-sm);
				font-weight: var(--fw-medium);
				color: var(--text-primary);
				line-height: var(--lh-snug);

				/* Multi-line truncation logic */
				display: -webkit-box;
				-webkit-line-clamp: 2;
				-webkit-box-orient: vertical;
				overflow: hidden;
				word-break: break-all;
			}

			.meta {
				display: flex;
				align-items: center;
				gap: var(--space-2);
				font-size: var(--fs-xs);
				color: var(--text-muted);
			}
		}

		.menu-container {
			position: absolute;
			top: var(--space-3);
			right: var(--space-3);

			&.visible .menu-btn {
				opacity: 1;
			}

			.menu-btn {
				background: transparent;
				border: none;
				color: var(--text-muted);
				cursor: pointer;
				padding: var(--space-1);
				border-radius: var(--radius-sm);
				opacity: 0;
				transition: color var(--dur) var(--ease), background var(--dur) var(--ease);

				&:hover {
					color: var(--text-primary);
					background: var(--tint-softer);
				}
			}

			.dropdown-menu {
				position: absolute;
				top: 100%;
				right: 0;
				background: var(--bg-elevated);
				border: 1px solid var(--border-default);
				border-radius: var(--radius-md);
				padding: var(--space-1);
				min-width: 140px;
				z-index: 100;
				box-shadow: var(--shadow-lg);
				display: flex;
				flex-direction: column;
				gap: 2px;
				margin-top: var(--space-1);

				.dropdown-item {
					display: flex;
					align-items: center;
					gap: var(--space-2);
					padding: var(--space-2) var(--space-3);
					background: transparent;
					border: none;
					color: var(--text-secondary);
					font-family: inherit;
					font-size: var(--fs-sm);
					cursor: pointer;
					border-radius: var(--radius-sm);
					text-align: left;
					width: 100%;
					transition: background var(--dur) var(--ease), color var(--dur) var(--ease);
					font-weight: var(--fw-medium);

					&:hover {
						background: var(--tint-soft);
						color: var(--text-primary);
					}

					&.danger {
						color: var(--danger);
						&:hover {
							background: rgba(255, 70, 85, 0.1);
						}
					}
				}
			}
		}
	}
</style>
