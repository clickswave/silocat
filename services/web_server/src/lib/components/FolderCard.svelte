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
							color={starred ? '#eab308' : 'inherit'}
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
		border-radius: 16px;
		padding: 16px;
		display: flex;
		flex-direction: column;
		justify-content: space-between;
		position: relative;
		cursor: pointer;
		transition: all 0.2s ease-out;
		min-height: 160px;
		backdrop-filter: blur(10px);

		&.compact {
			flex-direction: row;
			align-items: center;
			min-height: auto;
			padding: 12px 16px;
			gap: 12px;
			border-radius: 12px;

			.folder-content {
				flex-direction: row;
				align-items: center;
				gap: 12px;
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
			gap: 16px;
			flex: 1;
		}

		.icon-wrapper {
			display: flex;
			align-items: center;
			justify-content: flex-start;
			position: relative;

			gap: 12px;

			.folder-icon {
				color: var(--primary, #ff4655);
				filter: drop-shadow(0 4px 12px rgba(255, 70, 85, 0.3));
				opacity: 0.9;
			}

			.star-indicator {
				color: #eab308;
				opacity: 0.8;
				display: flex;
				align-items: center;
				justify-content: center;
			}
		}

		.folder-info {
			display: flex;
			flex-direction: column;
			gap: 4px;

			.name {
				font-size: 15px;
				font-weight: 500;
				color: var(--text-primary);
				white-space: nowrap;
				overflow: hidden;
				text-overflow: ellipsis;
				max-width: 100%;
			}

			.count {
				font-size: 12px;
				color: var(--text-muted);
			}
		}

		.menu-container {
			position: absolute;
			top: 12px;
			right: 12px;

			&.visible .menu-btn {
				opacity: 1;
			}

			.menu-btn {
				background: transparent;
				border: none;
				color: var(--text-muted);
				cursor: pointer;
				padding: 4px;
				border-radius: 6px;
				opacity: 0; /* Hidden by default */
				transition: all 0.2s;

				&:hover {
					color: var(--text-primary);
					background: var(--nav-hover);
				}
			}

			.dropdown-menu {
				position: absolute;
				top: 100%;
				right: 0;
				background: var(--bg-card);
				border: 1px solid var(--border-default);
				border-radius: 12px;
				padding: 6px;
				min-width: 160px;
				z-index: 100;
				box-shadow: 0 10px 30px rgba(0, 0, 0, 0.5);
				display: flex;
				flex-direction: column;
				gap: 2px;
				margin-top: 4px;

				.divider {
					height: 1px;
					background: var(--border-default);
					margin: 4px 0;
				}

				.dropdown-item {
					display: flex;
					align-items: center;
					gap: 10px;
					padding: 8px 12px;
					background: transparent;
					border: none;
					color: var(--text-secondary);
					font-size: 13px;
					cursor: pointer;
					border-radius: 8px;
					text-align: left;
					width: 100%;
					transition: all 0.1s;
					font-weight: 500;

					&:hover {
						background: var(--nav-hover);
						color: var(--text-primary);
					}

					&.danger {
						color: #ef4444;
						&:hover {
							background: rgba(239, 68, 68, 0.1);
						}
					}
				}
			}
		}
	}
</style>
