<script>
	import Icon from '@iconify/svelte';
	import { createQuery, useQueryClient } from '@tanstack/svelte-query';
	import { fade, fly } from 'svelte/transition';
	import { enhance } from '$app/forms';

	let { data } = $props();
	const queryClient = useQueryClient();

	let searchQuery = $state('');
	let showDeleteModal = $state(false);
	let userToDelete = $state(null);
	let deleteEmailConfirmation = $state('');

	const usersQuery = createQuery(() => ({
		queryKey: ['users'],
		queryFn: async () => {
			const res = await fetch('/api/users');
			if (!res.ok) throw new Error('Failed to fetch');
			const json = await res.json();
			return json.users || [];
		},
		initialData: data.users,
		refetchInterval: 5000
	}));

	let users = $derived(usersQuery.data || []);

	let filteredUsers = $derived.by(() => {
		if (!searchQuery) return users;
		const q = searchQuery.toLowerCase();
		return users.filter(
			(u) => u.username.toLowerCase().includes(q) || u.email.toLowerCase().includes(q)
		);
	});

	function openDeleteModal(user) {
		userToDelete = user;
		deleteEmailConfirmation = '';
		showDeleteModal = true;
	}

	function closeDeleteModal() {
		showDeleteModal = false;
		userToDelete = null;
	}

	function formatDate(dateStr) {
		if (!dateStr) return '-';
		return new Date(dateStr).toLocaleDateString();
	}

	function formatBytes(bytes) {
		if (bytes === 0) return '0 B';
		const k = 1024;
		const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
		const i = Math.floor(Math.log(bytes) / Math.log(k));
		return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
	}
</script>

<div class="page-header">
	<div class="header-content">
		<div>
			<h1>Users</h1>
			<p>View registered user details.</p>
		</div>
	</div>
</div>

<div class="card list">
	<div class="card-header">
		<div class="header-left">
			<h2>All Users</h2>
			<div class="badge">{filteredUsers.length} Total</div>
			{#if usersQuery.isFetching}
				<Icon icon="ri:loader-4-line" class="spin" />
			{/if}
		</div>
		<div class="header-right">
			<div class="search-box">
				<Icon icon="ri:search-line" class="search-icon" />
				<input type="text" placeholder="Search username or email..." bind:value={searchQuery} />
			</div>
		</div>
	</div>

	<div class="table-container">
		<table>
			<thead>
				<tr>
					<th>Username</th>
					<th>Email</th>
					<th>Account Type</th>
					<th>Storage</th>
					<th>Created On</th>
					<th>Verified</th>
					<th>Actions</th>
				</tr>
			</thead>
			<tbody>
				{#each filteredUsers as user}
					<tr>
						<td><span class="user-pill">{user.username}</span></td>
						<td class="muted">{user.email}</td>
						<td>
							<span class="type-pill" class:pro={user.account_type === 'enterprise'}>
								{user.account_type}
							</span>
						</td>
						<td class="muted">{formatBytes(user.default_storage_bytes)}</td>
						<td class="muted">{formatDate(user.created_on)}</td>
						<td>
							<span class="status-dot" class:active={user.email_verified}></span>
							{user.email_verified ? 'Yes' : 'No'}
						</td>
						<td class="actions">
							<button
								class="icon-btn delete"
								onclick={() => openDeleteModal(user)}
								aria-label="Delete user"
							>
								<Icon icon="ri:delete-bin-line" />
							</button>
						</td>
					</tr>
				{/each}
			</tbody>
		</table>
	</div>
</div>

{#if showDeleteModal && userToDelete}
	<div class="modal-backdrop" transition:fade={{ duration: 200 }}>
		<div class="modal" transition:fly={{ y: 20, duration: 300 }}>
			<div class="modal-header">
				<h2>Delete User</h2>
				<button class="close-btn" onclick={closeDeleteModal}><Icon icon="ri:close-line" /></button>
			</div>
			<div class="modal-body">
				<div class="warning-box">
					<Icon icon="ri:alert-fill" class="warning-icon" />
					<p>
						This action is irreversible. The user and all their data will be permanently deleted.
					</p>
				</div>
				<p>To confirm, please type <strong>{userToDelete.email}</strong> below:</p>

				<form
					method="POST"
					action="?/delete"
					use:enhance={() => {
						return async ({ result }) => {
							if (result.type === 'success') {
								closeDeleteModal();
								queryClient.invalidateQueries({ queryKey: ['users'] });
							} else {
								alert('Failed to delete user');
							}
						};
					}}
				>
					<input type="hidden" name="id" value={userToDelete.id} />
					<input
						type="email"
						class="confirm-input"
						placeholder={userToDelete.email}
						bind:value={deleteEmailConfirmation}
						autocomplete="off"
					/>

					<div class="modal-footer">
						<button type="button" class="btn secondary" onclick={closeDeleteModal}>Cancel</button>
						<button
							type="submit"
							class="btn danger"
							disabled={deleteEmailConfirmation !== userToDelete.email}
						>
							Delete User
						</button>
					</div>
				</form>
			</div>
		</div>
	</div>
{/if}

<style lang="scss">
	.spin {
		animation: spin 1s linear infinite;
	}
	@keyframes spin {
		100% {
			transform: rotate(360deg);
		}
	}

	.page-header {
		margin-bottom: 2rem;
		.header-content {
			display: flex;
			justify-content: space-between;
			align-items: center;
		}
		h1 {
			font-size: 2rem;
			font-weight: 700;
			margin: 0 0 0.5rem 0;
		}
		p {
			color: var(--text-muted);
			font-size: 1rem;
			margin: 0;
		}
	}

	.card {
		background: var(--bg-card);
		border: 1px solid var(--border-default);
		border-radius: var(--radius-lg);
		padding: 1.5rem;
		box-shadow: var(--shadow-card);
	}

	.list {
		.card-header {
			display: flex;
			justify-content: space-between;
			align-items: center;
			margin-bottom: 2rem;
			.header-left {
				display: flex;
				align-items: center;
				gap: 1rem;
			}
			h2 {
				margin: 0;
				font-size: 1.2rem;
				font-weight: 700;
				letter-spacing: -0.02em;
			}
			.badge {
				background: var(--bg-card-hover);
				padding: 0.25rem 0.75rem;
				border-radius: 20px;
				font-size: 0.8rem;
				color: var(--text-secondary);
				border: 1px solid var(--border-subtle);
			}
			.search-box {
				position: relative;
				.search-icon {
					position: absolute;
					left: 1rem;
					top: 50%;
					transform: translateY(-50%);
					color: var(--text-muted);
					pointer-events: none;
				}
				input {
					background: var(--bg-input);
					border: 1px solid var(--border-default);
					padding: 0.7rem 1rem 0.7rem 2.8rem;
					border-radius: var(--radius-md);
					color: var(--text-primary);
					font-family: inherit;
					outline: none;
					min-width: 300px;
					transition: all 0.2s;
					&:focus {
						border-color: var(--primary);
						box-shadow: 0 0 0 2px rgba(255, 26, 26, 0.1);
					}
					&::placeholder {
						color: var(--text-muted);
					}
				}
			}
		}
		.table-container {
			overflow-x: auto;
			table {
				width: 100%;
				border-collapse: separate;
				border-spacing: 0;
				text-align: left;
				thead th {
					padding: 1rem;
					color: var(--text-muted);
					font-weight: 600;
					font-size: 0.8rem;
					text-transform: uppercase;
					letter-spacing: 0.05em;
					border-bottom: 1px solid var(--border-default);
				}
				tbody tr {
					transition: background 0.1s;
					&:hover {
						background: var(--bg-card-hover);
					}
				}
				td {
					padding: 1.2rem 1rem;
					border-bottom: 1px solid var(--border-subtle);
					font-size: 0.95rem;
					color: var(--text-secondary);
				}
				tr:last-child td {
					border-bottom: none;
				}
				.user-pill {
					font-weight: 600;
					color: var(--text-primary);
				}
				.type-pill {
					font-size: 0.75rem;
					padding: 0.25rem 0.75rem;
					border-radius: 20px;
					background: var(--bg-card-hover);
					text-transform: uppercase;
					font-weight: 700;
					letter-spacing: 0.05em;
					border: 1px solid var(--border-subtle);
					&.pro {
						background: rgba(255, 26, 26, 0.1);
						color: var(--primary);
						border-color: rgba(255, 26, 26, 0.2);
					}
				}
				.muted {
					color: var(--text-muted);
					font-size: 0.85rem;
				}
				.status-dot {
					display: inline-block;
					width: 6px;
					height: 6px;
					border-radius: 50%;
					background: var(--text-muted);
					margin-right: 8px;
					vertical-align: middle;
					&.active {
						background: var(--success);
						box-shadow: 0 0 8px rgba(16, 185, 129, 0.4);
					}
				}
			}
		}
	}
	/* Actions & Modal Styles */
	.icon-btn {
		background: transparent;
		border: none;
		color: var(--text-muted);
		cursor: pointer;
		padding: 0.4rem;
		border-radius: 6px;
		transition: all 0.2s;
		&:hover {
			color: var(--text-primary);
			background: var(--bg-card-hover);
		}
		&.delete:hover {
			color: #ef4444;
			background: rgba(239, 68, 68, 0.1);
		}
	}

	.modal-backdrop {
		position: fixed;
		top: 0;
		left: 0;
		width: 100%;
		height: 100%;
		background: rgba(0, 0, 0, 0.6);
		z-index: 100;
		display: flex;
		align-items: center;
		justify-content: center;
		backdrop-filter: blur(4px);
	}

	.modal {
		background: var(--bg-card);
		border: 1px solid var(--border-default);
		border-radius: var(--radius-lg);
		width: 100%;
		max-width: 450px;
		padding: 1.5rem;
		box-shadow: 0 20px 25px -5px rgba(0, 0, 0, 0.3);

		.modal-header {
			display: flex;
			justify-content: space-between;
			align-items: center;
			margin-bottom: 1.5rem;
			h2 {
				margin: 0;
				font-size: 1.25rem;
				font-weight: 700;
			}
			.close-btn {
				background: transparent;
				border: none;
				color: var(--text-muted);
				cursor: pointer;
				font-size: 1.2rem;
				padding: 0.2rem;
				&:hover {
					color: var(--text-primary);
				}
			}
		}

		.modal-body {
			.warning-box {
				background: rgba(239, 68, 68, 0.1);
				border: 1px solid rgba(239, 68, 68, 0.2);
				border-radius: var(--radius-md);
				padding: 1rem;
				display: flex;
				gap: 0.8rem;
				align-items: flex-start;
				margin-bottom: 1.5rem;
				.warning-icon {
					color: #ef4444;
					font-size: 1.2rem;
					flex-shrink: 0;
					margin-top: 2px;
				}
				p {
					margin: 0;
					color: #fca5a5;
					font-size: 0.9rem;
					line-height: 1.4;
				}
			}
			p {
				color: var(--text-secondary);
				margin-bottom: 0.8rem;
				font-size: 0.95rem;
			}

			.confirm-input {
				width: 100%;
				background: var(--bg-input);
				border: 1px solid var(--border-default);
				padding: 0.8rem;
				border-radius: var(--radius-md);
				color: var(--text-primary);
				margin-bottom: 1.5rem;
				outline: none;
				transition: all 0.2s;
				&:focus {
					border-color: #ef4444;
					box-shadow: 0 0 0 2px rgba(239, 68, 68, 0.1);
				}
			}
		}

		.modal-footer {
			display: flex;
			justify-content: flex-end;
			gap: 0.8rem;
			.btn {
				padding: 0.6rem 1.2rem;
				border-radius: var(--radius-md);
				font-weight: 600;
				cursor: pointer;
				transition: all 0.2s;
				border: none;
				font-size: 0.9rem;
				&.secondary {
					background: transparent;
					border: 1px solid var(--border-default);
					color: var(--text-secondary);
					&:hover {
						background: var(--bg-card-hover);
						color: var(--text-primary);
					}
				}
				&.danger {
					background: #ef4444;
					color: white;
					&:hover:not(:disabled) {
						background: #dc2626;
						box-shadow: 0 0 10px rgba(239, 68, 68, 0.3);
					}
					&:disabled {
						opacity: 0.5;
						cursor: not-allowed;
					}
				}
			}
		}
	}
</style>
