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
							color={starred ? '#eab308' : 'inherit'}
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
		border-radius: 16px;
		padding: 16px;
		display: flex;
		flex-direction: column;
		justify-content: space-between;
		position: relative;
		cursor: pointer;
		min-height: 160px;
		backdrop-filter: blur(10px);
		transition: all 0.2s ease-out;

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
			gap: 16px;
		}

		.icon-section {
			display: flex;
			align-items: flex-start;
			justify-content: flex-start;
			gap: 12px;

			.file-icon {
				color: var(--text-muted);
				transition: color 0.2s;
				opacity: 0.8;

				&.image {
					color: #facc15;
				} /* Yellow-ish */
				&.video {
					color: #f43f5e;
				} /* Rose */
				&.doc {
					color: #60a5fa;
				} /* Blue */
				&.audio {
					color: #a78bfa;
				} /* Purple */
			}

			.indicators {
				display: flex;
				gap: 4px;
				align-items: center;

				.lock-indicator {
					color: #facc15;
					opacity: 0.8;
				}
				.star-indicator {
					color: #eab308;
					opacity: 0.8;
				}
			}
		}

		.info-section {
			margin-top: auto;
			display: flex;
			flex-direction: column;
			gap: 4px;

			.name {
				font-size: 15px;
				font-weight: 500;
				color: var(--text-primary);
				line-height: 1.4;

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
				gap: 8px;
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
				color: rgba(255, 255, 255, 0.4);
				cursor: pointer;
				padding: 4px;
				border-radius: 6px;
				opacity: 0;
				transition: all 0.2s;

				&:hover {
					color: white;
					background: rgba(255, 255, 255, 0.1);
				}
			}

			.dropdown-menu {
				position: absolute;
				top: 100%;
				right: 0;
				background: #18181b;
				border: 1px solid rgba(255, 255, 255, 0.1);
				border-radius: 12px;
				padding: 6px;
				min-width: 140px;
				z-index: 100;
				box-shadow: 0 10px 30px rgba(0, 0, 0, 0.5);
				display: flex;
				flex-direction: column;
				gap: 2px;

				.dropdown-item {
					display: flex;
					align-items: center;
					gap: 10px;
					padding: 8px 12px;
					background: transparent;
					border: none;
					color: #d4d4d8;
					font-size: 13px;
					cursor: pointer;
					border-radius: 8px;
					text-align: left;
					width: 100%;
					transition: all 0.1s;
					font-weight: 500;

					&:hover {
						background: rgba(255, 255, 255, 0.05);
						color: white;
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
