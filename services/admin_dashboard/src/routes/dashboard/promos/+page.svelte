<script>
	import Icon from '@iconify/svelte';
	import { enhance } from '$app/forms';
	import { createQuery } from '@tanstack/svelte-query';

	let { data, form } = $props();

	let showModal = $state(false);
	let creating = $state(false);

	let searchQuery = $state('');

	const promosQuery = createQuery(() => ({
		queryKey: ['promos'],
		queryFn: async () => {
			const res = await fetch('/api/promos');
			if (!res.ok) throw new Error('Failed to fetch');
			const json = await res.json();
			return json.promos || [];
		},
		initialData: data.promos,
		refetchInterval: 5000
	}));

	let promos = $derived(promosQuery.data || []);

	// Close modal on success
	$effect(() => {
		if (form?.success) {
			showModal = false;
			creating = false;
			promosQuery.refetch();
		}
	});

	let filteredPromos = $derived.by(() => {
		if (!searchQuery) return promos;
		const q = searchQuery.toLowerCase();
		return promos.filter((p) => p.code.toLowerCase().includes(q));
	});

	function copyToClipboard(text) {
		navigator.clipboard.writeText(text);
		// In a real app, trigger a toast notification here
		alert('Copied to clipboard');
	}
</script>

<div class="page-header">
	<div class="header-content">
		<div>
			<h1>Promo Codes</h1>
			<p>Manage discount codes for subscriptions.</p>
		</div>
		<button class="primary-btn" onclick={() => (showModal = true)}>
			<Icon icon="ri:add-line" />
			Create Promo
		</button>
	</div>
</div>

<div class="card list">
	<div class="card-header">
		<div class="header-left">
			<h2>All Promos</h2>
			<div class="badge">{filteredPromos.length} Total</div>
			{#if promosQuery.isFetching}
				<Icon icon="ri:loader-4-line" class="spin" />
			{/if}
		</div>
		<div class="header-right">
			<div class="search-box">
				<Icon icon="ri:search-line" class="search-icon" />
				<input type="text" placeholder="Search code..." bind:value={searchQuery} />
			</div>
		</div>
	</div>

	<div class="table-container">
		<table>
			<thead>
				<tr>
					<th>Code</th>
					<th>Discount</th>
					<th>Duration</th>
					<th>Active</th>
					<th>Action</th>
				</tr>
			</thead>
			<tbody>
				{#each filteredPromos as promo}
					<tr>
						<td><code class="code-pill">{promo.code}</code></td>
						<td><span class="type-pill pro">{promo.discount_percentage}% OFF</span></td>
						<td class="muted">{promo.duration}</td>
						<td>
							<span class="status-dot" class:active={promo.active}></span>
							{promo.active ? 'Active' : 'Inactive'}
						</td>
						<td>
							<button class="icon-btn" onclick={() => copyToClipboard(promo.code)}>
								<Icon icon="ri:file-copy-line" />
							</button>
						</td>
					</tr>
				{/each}
			</tbody>
		</table>
	</div>
</div>

{#if showModal}
	<div
		class="modal-backdrop"
		onclick={() => (showModal = false)}
		role="button"
		tabindex="0"
		onkeydown={(e) => e.key === 'Escape' && (showModal = false)}
	>
		<div
			class="modal"
			onclick={(e) => e.stopPropagation()}
			role="button"
			tabindex="0"
			onkeydown={(e) => {}}
		>
			<div class="modal-header">
				<h3>Create Promo Code</h3>
				<button class="close-btn" onclick={() => (showModal = false)}>
					<Icon icon="ri:close-line" />
				</button>
			</div>

			<form
				method="POST"
				action="?/create"
				use:enhance={() => {
					creating = true;
					return async ({ update }) => {
						await update();
						creating = false;
					};
				}}
			>
				<div class="form-group">
					<label for="code">Promo Code</label>
					<input type="text" id="code" name="code" placeholder="e.g. SUMMER-SALE" required />
				</div>

				<div class="form-group">
					<label for="discount">Discount Percentage</label>
					<input
						type="number"
						id="discount"
						name="discount"
						placeholder="e.g. 20"
						min="1"
						max="100"
						required
					/>
				</div>

				<div class="form-group">
					<label for="duration">Duration</label>
					<select id="duration" name="duration">
						<option value="once">Once</option>
						<option value="forever">Forever</option>
						<option value="1 month">1 Month</option>
						<option value="3 months">3 Months</option>
						<option value="1 year">1 Year</option>
					</select>
				</div>

				<div class="modal-actions">
					<button type="button" class="secondary-btn" onclick={() => (showModal = false)}
						>Cancel</button
					>
					<button type="submit" class="primary-btn" disabled={creating}>
						{#if creating}Creating...{:else}Create Promo{/if}
					</button>
				</div>
			</form>
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

	.primary-btn {
		background: var(--primary);
		color: black;
		border: none;
		padding: 0.8rem 1.5rem;
		border-radius: var(--radius-md);
		font-weight: 700;
		display: flex;
		align-items: center;
		gap: 0.5rem;
		cursor: pointer;
		transition: all 0.2s ease-out;
		font-size: 0.95rem;
		&:hover:not(:disabled) {
			background: var(--primary-hover);
			box-shadow: 0 0 15px var(--primary-glow);
			transform: translateY(-1px);
			color: white;
		}
		&:disabled {
			opacity: 0.5;
			cursor: not-allowed;
		}
	}

	.secondary-btn {
		background: transparent;
		color: var(--text-secondary);
		border: 1px solid var(--border-default);
		padding: 0.8rem 1.5rem;
		border-radius: var(--radius-md);
		font-weight: 600;
		cursor: pointer;
		transition: all 0.2s;
		&:hover {
			border-color: var(--text-primary);
			color: var(--text-primary);
			background: var(--bg-card-hover);
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
				.icon-btn {
					background: transparent;
					border: none;
					color: var(--text-muted);
					cursor: pointer;
					padding: 6px;
					border-radius: 6px;
					transition: all 0.2s;
					&:hover {
						color: var(--text-primary);
						background: var(--bg-card-hover);
					}
				}
			}
		}
	}

	.modal-backdrop {
		position: fixed;
		top: 0;
		left: 0;
		width: 100%;
		height: 100%;
		background: rgba(0, 0, 0, 0.8);
		backdrop-filter: blur(8px);
		display: flex;
		justify-content: center;
		align-items: center;
		z-index: 100;
	}
	.modal {
		background: var(--bg-card);
		border: 1px solid var(--border-default);
		border-radius: var(--radius-lg);
		padding: 2.5rem;
		width: 100%;
		max-width: 480px;
		box-shadow: 0 20px 50px rgba(0, 0, 0, 0.5);
		animation: modalPop 0.3s cubic-bezier(0.16, 1, 0.3, 1);
		.modal-header {
			display: flex;
			justify-content: space-between;
			align-items: center;
			margin-bottom: 2.5rem;
			h3 {
				margin: 0;
				font-size: 1.5rem;
				font-weight: 700;
				color: var(--text-primary);
			}
			.close-btn {
				background: transparent;
				border: none;
				color: var(--text-muted);
				cursor: pointer;
				padding: 4px;
				transition: color 0.2s;
				&:hover {
					color: var(--text-primary);
				}
			}
		}
		.form-group {
			display: flex;
			flex-direction: column;
			gap: 0.6rem;
			margin-bottom: 2rem;
			label {
				font-size: 0.85rem;
				color: var(--text-secondary);
				font-weight: 500;
				margin-left: 2px;
			}
			select,
			input {
				background: var(--bg-input);
				border: 1px solid var(--border-default);
				padding: 1rem 1.2rem;
				border-radius: var(--radius-md);
				color: var(--text-primary);
				font-family: inherit;
				outline: none;
				font-size: 0.95rem;
				transition: all 0.2s;
				&:focus {
					border-color: var(--primary);
					box-shadow: 0 0 0 1px var(--primary);
					background: var(--bg-card-hover);
				}
			}
		}
		.modal-actions {
			display: flex;
			justify-content: flex-end;
			gap: 1rem;
			margin-top: 3rem;
		}
	}
	@keyframes modalPop {
		from {
			opacity: 0;
			transform: scale(0.95) translateY(10px);
		}
		to {
			opacity: 1;
			transform: scale(1) translateY(0);
		}
	}
</style>
