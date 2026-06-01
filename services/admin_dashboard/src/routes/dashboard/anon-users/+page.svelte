<script>
	import Icon from '@iconify/svelte';
	import { createQuery } from '@tanstack/svelte-query';

	let { data } = $props();

	let searchQuery = $state('');

	const usersQuery = createQuery(() => ({
		queryKey: ['anon-users'],
		queryFn: async () => {
			const res = await fetch('/api/anon-users');
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
			(u) => u.ip_address.toLowerCase().includes(q) || u.api_key.toLowerCase().includes(q)
		);
	});

	function formatDate(dateStr) {
		if (!dateStr) return '-';
		return new Date(dateStr).toLocaleString();
	}
</script>

<div class="page-header">
	<div class="header-content">
		<div>
			<h1>Anonymous Users</h1>
			<p>View activity from anonymous sessions.</p>
		</div>
	</div>
</div>

<div class="card list">
	<div class="card-header">
		<div class="header-left">
			<h2>All Sessions</h2>
			<div class="badge">{filteredUsers.length} Total</div>
			{#if usersQuery.isFetching}
				<Icon icon="ri:loader-4-line" class="spin" />
			{/if}
		</div>
		<div class="header-right">
			<div class="search-box">
				<Icon icon="ri:search-line" class="search-icon" />
				<input type="text" placeholder="Search IP or API Key..." bind:value={searchQuery} />
			</div>
		</div>
	</div>

	<div class="table-container">
		<table>
			<thead>
				<tr>
					<th>IP Address</th>
					<th>API Key</th>
					<th>Last Seen</th>
					<th>Location</th>
				</tr>
			</thead>
			<tbody>
				{#each filteredUsers as user}
					<tr>
						<td><span class="user-pill">{user.ip_address}</span></td>
						<td><code class="code-pill">{user.api_key.substring(0, 8)}...</code></td>
						<td class="muted">{formatDate(user.last_seen)}</td>
						<td class="muted">
							{#if user.geo_location}
								{user.geo_location.city || '-'}, {user.geo_location.country || '-'}
							{:else}
								-
							{/if}
						</td>
					</tr>
				{/each}
			</tbody>
		</table>
	</div>
</div>

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
				.code-pill {
					font-family: 'JetBrains Mono', monospace;
					background: rgba(255, 255, 255, 0.05);
					color: var(--text-primary);
					padding: 0.4rem 0.8rem;
					border-radius: 6px;
					font-size: 0.85rem;
					border: 1px solid transparent;
					transition: all 0.2s;
					&:hover {
						border-color: var(--border-default);
						background: var(--bg-card-hover);
					}
				}
				.muted {
					color: var(--text-muted);
					font-size: 0.85rem;
				}
			}
		}
	}
</style>
