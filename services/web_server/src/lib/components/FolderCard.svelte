<script>
	import Icon from '@iconify/svelte';

	export let name = 'Folder';
	export let count = 0;
	// export let color = 'blue'; // Deprecated
	export let compact = false;
	export let starred = false;

	// New Props for Callbacks
	export let onrename = () => {};
	export let ondelete = () => {};
	export let ondownloadzip = () => {};
	export let onclick = () => {};
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

	function handleRename(e) {
		e.stopPropagation();
		closeMenu();
		if (onrename) onrename();
	}

	function handleDelete(e) {
		e.stopPropagation();
		closeMenu();
		if (ondelete) ondelete();
	}

	function handleDownloadZip(e) {
		e.stopPropagation();
		closeMenu();
		if (ondownloadzip) ondownloadzip();
	}

	function handleStar(e) {
		e.stopPropagation();
		// closeMenu();
		if (onstar) onstar();
	}

	function handleShare(e) {
		e.stopPropagation();
		console.log('FolderCard: Share clicked');
		closeMenu();
		if (onshare) onshare();
	}

	function handleRestore(e) {
		e.stopPropagation();
		closeMenu();
		if (onrestore) onrestore();
	}

	// Simple click outside handling
	function handleClickOutside(event) {
		if (showMenu && menuRef && !menuRef.contains(event.target)) {
			closeMenu();
		}
	}
</script>

<svelte:window on:click={handleClickOutside} />

<!-- svelte-ignore a11y-click-events-have-key-events -->
<div class="folder-card {compact ? 'compact' : ''}" on:click={onclick} role="button" tabindex="0">
	<div class="folder-content">
		<div class="icon-wrapper">
			<Icon icon="ri:folder-5-fill" class="folder-icon" width={compact ? '20' : '32'} />
			{#if starred}
				<div class="star-indicator" title="Starred">
					<Icon icon="ri:star-fill" width="14" />
				</div>
			{/if}
		</div>
		<div class="folder-info">
			<span class="name" title={name}>{name}</span>
			<span class="count">{count} items</span>
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
					<button class="dropdown-item" on:click={handleRename}>
						<Icon icon="ri:edit-line" width="16" />
						Rename
					</button>
					<button class="dropdown-item" on:click={handleDownloadZip}>
						<Icon icon="ri:file-zip-line" width="16" />
						Download Zip
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

					<div class="divider"></div>
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
	.folder-card {
		background: var(--bg-card);
		border: 1px solid var(--border-default);
		border-radius: var(--radius-md);
		padding: var(--space-4);
		display: flex;
		flex-direction: column;
		justify-content: space-between;
		position: relative;
		cursor: pointer;
		transition: background var(--dur) var(--ease), border-color var(--dur) var(--ease),
			box-shadow var(--dur) var(--ease);
		min-height: 160px;
		box-shadow: var(--shadow-card);

		&.compact {
			flex-direction: row;
			align-items: center;
			min-height: auto;
			padding: var(--space-3) var(--space-4);
			gap: var(--space-3);
			border-radius: var(--radius-md);

			.folder-content {
				flex-direction: row;
				align-items: center;
				gap: var(--space-3);
			}

			.folder-info {
				align-items: flex-start;
			}
		}

		&:hover {
			background: var(--bg-card-hover);
			border-color: var(--border-active);
			box-shadow: var(--shadow-card);

			.menu-container .menu-btn {
				opacity: 1;
			}
		}

		.folder-content {
			display: flex;
			flex-direction: column;
			gap: var(--space-4);
			flex: 1;
		}

		.icon-wrapper {
			display: flex;
			align-items: center;
			justify-content: flex-start;
			position: relative;

			gap: var(--space-3);

			.folder-icon {
				color: var(--primary);
				opacity: 0.9;
			}

			.star-indicator {
				color: var(--warning);
				opacity: 0.85;
				display: flex;
				align-items: center;
				justify-content: center;
			}
		}

		.folder-info {
			display: flex;
			flex-direction: column;
			gap: var(--space-1);

			.name {
				font-size: var(--fs-sm);
				font-weight: var(--fw-medium);
				color: var(--text-primary);
				white-space: nowrap;
				overflow: hidden;
				text-overflow: ellipsis;
				max-width: 100%;
			}

			.count {
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
				opacity: 0; /* Hidden by default */
				transition: color var(--dur) var(--ease), background var(--dur) var(--ease);

				&:hover {
					color: var(--text-primary);
					background: var(--nav-hover);
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
				min-width: 160px;
				z-index: 100;
				box-shadow: var(--shadow-lg);
				display: flex;
				flex-direction: column;
				gap: 2px;
				margin-top: var(--space-1);

				.divider {
					height: 1px;
					background: var(--border-default);
					margin: var(--space-1) 0;
				}

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
						background: var(--nav-hover);
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
