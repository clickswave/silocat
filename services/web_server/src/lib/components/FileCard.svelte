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
	<div class="top">
		<div class="file-icon {type}">
			{#if type === 'image'}<Icon icon="ri:image-2-line" width="20" />
			{:else if type === 'video'}<Icon icon="ri:film-line" width="20" />
			{:else if type === 'doc'}<Icon icon="ri:file-text-line" width="20" />
			{:else if type === 'audio'}<Icon icon="ri:music-2-line" width="20" />
			{:else}<Icon icon="ri:file-3-line" width="20" />
			{/if}
		</div>
		<div class="indicators">
			{#if starred}
				<span class="ind star" title="Starred"><Icon icon="ri:star-fill" width="13" /></span>
			{/if}
			{#if encrypted}
				<span class="ind" title="Encrypted"><Icon icon="ri:lock-2-line" width="13" /></span>
			{/if}
		</div>
	</div>

	<div class="info-section">
		<span class="name" title={name}>{name}</span>
		<div class="meta">
			<span class="size">{size}</span>
			{#if date}<span class="date">{date}</span>{/if}
		</div>
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

		.file-icon {
			display: flex;
			align-items: center;
			justify-content: center;
			width: 40px;
			height: 40px;
			border: 1px solid var(--edge);
			border-radius: var(--radius-sm);
			color: var(--ink-faint);
			flex-shrink: 0;
			transition: color var(--dur) var(--ease);

			&.image,
			&.video {
				color: var(--ink-mute);
			}
		}

		.indicators {
			display: flex;
			gap: var(--space-1);
			align-items: center;
			/* keep clear of the hover kebab in the corner */
			margin-right: var(--space-6);

			.ind {
				display: flex;
				color: var(--ink-faint);

				&.star {
					color: var(--warn);
				}
			}
		}

		.info-section {
			display: flex;
			flex-direction: column;
			gap: var(--space-1);
			margin-top: auto;

			.name {
				font-size: var(--fs-sm);
				font-weight: var(--fw-medium);
				color: var(--ink);
				line-height: var(--lh-snug);
				display: -webkit-box;
				-webkit-line-clamp: 2;
				-webkit-box-orient: vertical;
				overflow: hidden;
				word-break: break-word;
				/* reserve two lines so single-line names align across the row */
				min-height: calc(2 * var(--fs-sm) * var(--lh-snug));
			}

			.meta {
				display: flex;
				align-items: center;
				gap: var(--space-2);
				font-family: var(--font-mono);
				font-size: var(--fs-xs);
				color: var(--ink-faint);

				.date::before {
					content: '·';
					margin-right: var(--space-2);
				}
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
				min-width: 150px;
				z-index: 100;
				box-shadow: var(--shadow-overlay);
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
