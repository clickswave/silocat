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
	<div class="top">
		<div class="folder-icon">
			<Icon icon="ri:folder-3-line" width={compact ? '17' : '20'} />
		</div>
		{#if starred && !compact}
			<span class="ind star" title="Starred"><Icon icon="ri:star-fill" width="13" /></span>
		{/if}
	</div>
	<div class="folder-info">
		<span class="name" title={name}>{name}</span>
		<span class="count">{count} {count === 1 ? 'item' : 'items'}</span>
	</div>

	<div class="menu-container {showMenu ? 'visible' : ''}" bind:this={menuRef}>
		<button class="menu-btn" on:click={toggleMenu} aria-label="More actions">
			<Icon icon="ri:more-2-fill" width="18" />
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
		background: var(--surface);
		border: 1px solid var(--edge);
		border-radius: var(--radius-md);
		padding: var(--space-4);
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
		height: 100%;
		position: relative;
		cursor: pointer;
		transition:
			background var(--dur) var(--ease),
			border-color var(--dur) var(--ease);

		&.compact {
			flex-direction: row;
			align-items: center;
			padding: var(--space-3);
			gap: var(--space-3);
			height: auto;

			.folder-info {
				gap: 0;
				flex: 1;
				margin-top: 0;
			}
		}

		&:hover {
			background: var(--surface-hover);
			border-color: var(--edge-strong);

			.menu-container .menu-btn {
				opacity: 1;
			}
		}

		.top {
			display: flex;
			align-items: flex-start;
			justify-content: space-between;
			gap: var(--space-2);
		}

		.folder-icon {
			display: flex;
			align-items: center;
			justify-content: center;
			width: 40px;
			height: 40px;
			border: 1px solid var(--edge);
			border-radius: var(--radius-sm);
			color: var(--ink-mute);
			flex-shrink: 0;
		}

		&.compact .folder-icon {
			width: 32px;
			height: 32px;
		}

		.ind {
			display: flex;
			color: var(--ink-faint);
			margin-right: var(--space-6);

			&.star {
				color: var(--warn);
			}
		}

		.folder-info {
			display: flex;
			flex-direction: column;
			gap: var(--space-1);
			min-width: 0;
			margin-top: auto;

			.name {
				font-size: var(--fs-sm);
				font-weight: var(--fw-medium);
				color: var(--ink);
				white-space: nowrap;
				overflow: hidden;
				text-overflow: ellipsis;
				max-width: 100%;
			}

			.count {
				font-family: var(--font-mono);
				font-size: var(--fs-xs);
				color: var(--ink-faint);
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
				display: flex;
				background: transparent;
				border: none;
				color: var(--ink-faint);
				cursor: pointer;
				padding: var(--space-1);
				border-radius: var(--radius-sm);
				opacity: 0;
				transition:
					color var(--dur) var(--ease),
					background var(--dur) var(--ease),
					opacity var(--dur) var(--ease);

				&:hover {
					color: var(--ink);
					background: var(--tint-soft);
				}
			}

			.dropdown-menu {
				position: absolute;
				top: 100%;
				right: 0;
				background: var(--raised);
				border: 1px solid var(--edge);
				border-radius: var(--radius-md);
				padding: var(--space-1);
				min-width: 160px;
				z-index: 100;
				box-shadow: var(--shadow-overlay);
				display: flex;
				flex-direction: column;
				gap: 2px;
				margin-top: var(--space-1);

				.divider {
					height: 1px;
					background: var(--edge);
					margin: var(--space-1) 0;
				}

				.dropdown-item {
					display: flex;
					align-items: center;
					gap: var(--space-2);
					padding: var(--space-2) var(--space-3);
					background: transparent;
					border: none;
					color: var(--ink);
					font-family: inherit;
					font-size: var(--fs-sm);
					cursor: pointer;
					border-radius: var(--radius-sm);
					text-align: left;
					width: 100%;
					transition: background var(--dur-fast) var(--ease);

					:global(.iconify) {
						color: var(--ink-faint);
					}

					&:hover {
						background: var(--tint-soft);
					}

					&.danger {
						color: var(--danger);
						:global(.iconify) {
							color: var(--danger);
						}
						&:hover {
							background: var(--danger-soft);
						}
					}
				}
			}
		}
	}
</style>
