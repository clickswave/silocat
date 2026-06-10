<script>
	import Icon from '@iconify/svelte';
	import { page } from '$app/stores';
	import { toast } from 'svelte-sonner';
	import { onMount } from 'svelte';

	const id = $page.params.id;

	const categories = {
		help: 'Need help',
		suggestion: 'Suggestion',
		bug: 'Report a bug',
		account: 'Account / billing',
		other: 'Something else'
	};
	const catLabel = (c) => categories[c] || c;

	let ticket = $state(null);
	let replies = $state([]);
	let loading = $state(true);
	let notFound = $state(false);
	let body = $state('');
	let sending = $state(false);

	function fmt(iso) {
		try {
			return new Date(iso).toLocaleString(undefined, {
				month: 'short',
				day: 'numeric',
				hour: '2-digit',
				minute: '2-digit'
			});
		} catch {
			return '';
		}
	}

	async function load() {
		loading = true;
		try {
			const res = await fetch(`/api/v1/user/support/${id}`);
			if (res.status === 404) {
				notFound = true;
				return;
			}
			const d = await res.json();
			ticket = d?.success?.data?.ticket ?? null;
			replies = d?.success?.data?.replies ?? [];
			if (!ticket) notFound = true;
		} catch (e) {
			console.error(e);
			notFound = true;
		} finally {
			loading = false;
		}
	}

	let statusBusy = $state(false);
	async function setStatus(status) {
		if (statusBusy) return;
		statusBusy = true;
		try {
			const res = await fetch(`/api/v1/user/support/${id}/status`, {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({ status })
			});
			if (res.ok) {
				toast.success(status === 'closed' ? 'Ticket closed' : 'Ticket reopened');
				await load();
			} else {
				toast.error('Could not update ticket');
			}
		} catch (err) {
			console.error(err);
			toast.error('Could not update ticket');
		} finally {
			statusBusy = false;
		}
	}

	async function sendReply(e) {
		e.preventDefault();
		if (body.trim().length < 2 || sending) return;
		sending = true;
		try {
			const res = await fetch(`/api/v1/user/support/${id}/reply`, {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({ body: body.trim() })
			});
			const d = await res.json();
			if (!res.ok) {
				toast.error(d.message || d.error || 'Could not send reply');
			} else {
				body = '';
				await load();
			}
		} catch (err) {
			console.error(err);
			toast.error('Could not send reply');
		} finally {
			sending = false;
		}
	}

	onMount(load);
</script>

<div class="thread-page">
	<a class="back" href="/home/support"><Icon icon="ri:arrow-left-line" width="18" /> All tickets</a>

	{#if loading}
		<div class="state"><Icon icon="svg-spinners:ring-resize" width="28" /></div>
	{:else if notFound}
		<div class="state empty">
			<Icon icon="ri:error-warning-line" width="40" />
			<p>Ticket not found.</p>
			<a class="link" href="/home/support">Back to support</a>
		</div>
	{:else}
		<header class="t-header">
			<div class="t-head-main">
				<div class="t-head-top">
					<span class="t-cat">{catLabel(ticket.category)}</span>
					<span class="t-status {ticket.status === 'open' ? 'open' : 'done'}">
						{ticket.status === 'open' ? 'Open' : 'Resolved'}
					</span>
				</div>
				<h1>{ticket.subject}</h1>
				<p class="t-meta">Opened {fmt(ticket.created_at)}</p>
			</div>
			<button
				class="status-btn"
				disabled={statusBusy}
				onclick={() => setStatus(ticket.status === 'open' ? 'closed' : 'open')}
			>
				<Icon icon={ticket.status === 'open' ? 'ri:check-line' : 'ri:refresh-line'} width="16" />
				{ticket.status === 'open' ? 'Close ticket' : 'Reopen'}
			</button>
		</header>

		<div class="conversation">
			<!-- Opening message -->
			<div class="msg user">
				<div class="msg-head"><span class="who">You</span><span class="at">{fmt(ticket.created_at)}</span></div>
				<div class="bubble">{ticket.message}</div>
			</div>

			{#each replies as r (r.id)}
				<div class="msg {r.author_role === 'admin' ? 'admin' : 'user'}">
					<div class="msg-head">
						<span class="who">
							{#if r.author_role === 'admin'}
								<span class="admin-badge"><Icon icon="ri:shield-star-line" width="13" /> SiloCat Team</span>
							{:else}
								You
							{/if}
						</span>
						<span class="at">{fmt(r.created_at)}</span>
					</div>
					<div class="bubble">{r.body}</div>
				</div>
			{/each}
		</div>

		<form class="reply-box" onsubmit={sendReply}>
			<textarea
				rows="3"
				placeholder={ticket.status === 'closed' ? 'Reply to reopen this ticket…' : 'Write a reply…'}
				bind:value={body}
			></textarea>
			<div class="reply-actions">
				{#if ticket.status === 'closed'}
					<span class="hint">Sending a reply will reopen this ticket.</span>
				{/if}
				<button type="submit" class="send" disabled={body.trim().length < 2 || sending}>
					<Icon icon={sending ? 'svg-spinners:ring-resize' : 'ri:send-plane-2-line'} width="18" />
					{sending ? 'Sending…' : 'Send reply'}
				</button>
			</div>
		</form>
	{/if}
</div>

<style lang="scss">
	.thread-page {
		width: 100%;
		max-width: 760px;
		color: var(--text-primary);
		display: flex;
		flex-direction: column;
		gap: var(--space-5);
	}
	.back {
		display: inline-flex;
		align-items: center;
		gap: var(--space-1);
		color: var(--text-muted);
		font-size: var(--fs-sm);
		width: fit-content;
		&:hover {
			color: var(--text-primary);
		}
	}
	.state {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: var(--space-3);
		padding: var(--space-9) 0;
		color: var(--text-muted);
		.link {
			color: var(--primary);
			font-weight: var(--fw-medium);
			font-size: var(--fs-sm);
		}
	}
	.t-header {
		display: flex;
		align-items: flex-start;
		justify-content: space-between;
		gap: var(--space-4);
		flex-wrap: wrap;
		.t-head-top {
			display: flex;
			align-items: center;
			gap: var(--space-2);
			margin-bottom: var(--space-2);
		}
		.t-cat {
			color: var(--text-muted);
			text-transform: uppercase;
			letter-spacing: 0.04em;
			font-family: var(--font-mono);
			font-size: var(--fs-xs);
		}
		.t-status {
			font-size: 0.62rem;
			font-weight: var(--fw-semibold);
			text-transform: uppercase;
			letter-spacing: 0.04em;
			padding: 2px 8px;
			border-radius: 999px;
			&.open {
				background: color-mix(in srgb, var(--success, #3ddc97) 16%, transparent);
				color: var(--success, #3ddc97);
			}
			&.done {
				background: var(--tint-soft);
				color: var(--text-muted);
			}
		}
		h1 {
			font-size: var(--fs-h3);
			font-weight: var(--fw-semibold);
			margin: 0 0 var(--space-1);
		}
		.t-meta {
			margin: 0;
			color: var(--text-muted);
			font-size: var(--fs-xs);
			font-family: var(--font-mono);
		}
	}
	.status-btn {
		display: inline-flex;
		align-items: center;
		gap: var(--space-1);
		flex-shrink: 0;
		background: var(--tint-soft);
		border: 1px solid var(--border-default);
		color: var(--text-secondary);
		border-radius: var(--radius-pill, 999px);
		padding: var(--space-2) var(--space-4);
		font-family: inherit;
		font-size: var(--fs-sm);
		font-weight: var(--fw-medium);
		cursor: pointer;
		transition: color var(--dur) var(--ease), border-color var(--dur) var(--ease);
		&:hover:not(:disabled) {
			color: var(--text-primary);
			border-color: var(--border-active, var(--primary));
		}
		&:disabled {
			opacity: 0.6;
			cursor: default;
		}
	}
	.conversation {
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
	}
	.msg {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
		max-width: 86%;
		&.user {
			align-self: flex-end;
			align-items: flex-end;
		}
		&.admin {
			align-self: flex-start;
			align-items: flex-start;
		}
	}
	.msg-head {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		font-size: var(--fs-xs);
		color: var(--text-muted);
		.who {
			font-weight: var(--fw-semibold);
			color: var(--text-secondary);
		}
		.admin-badge {
			display: inline-flex;
			align-items: center;
			gap: 3px;
			color: var(--primary);
		}
		.at {
			font-family: var(--font-mono);
			color: var(--text-dim);
		}
	}
	.bubble {
		padding: var(--space-3) var(--space-4);
		border-radius: var(--radius-md);
		font-size: var(--fs-sm);
		line-height: var(--lh-normal, 1.55);
		white-space: pre-wrap;
		word-break: break-word;
	}
	.msg.user .bubble {
		background: color-mix(in srgb, var(--primary) 14%, var(--bg-card));
		border: 1px solid color-mix(in srgb, var(--primary) 30%, transparent);
		border-bottom-right-radius: 4px;
	}
	.msg.admin .bubble {
		background: var(--bg-card);
		border: 1px solid var(--border-default);
		border-bottom-left-radius: 4px;
	}
	.reply-box {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
		border-top: 1px solid var(--hairline);
		padding-top: var(--space-4);
		textarea {
			width: 100%;
			background: var(--bg-input);
			border: 1px solid var(--border-default);
			border-radius: var(--radius-sm);
			padding: 0.75rem 0.95rem;
			color: var(--text-primary);
			font-family: inherit;
			font-size: var(--fs-body);
			outline: none;
			resize: vertical;
			min-height: 80px;
			&:focus {
				border-color: var(--primary);
				box-shadow: 0 0 0 3px var(--primary-glow);
			}
		}
	}
	.reply-actions {
		display: flex;
		align-items: center;
		justify-content: flex-end;
		gap: var(--space-3);
		.hint {
			font-size: var(--fs-xs);
			color: var(--text-muted);
			margin-right: auto;
		}
	}
	.send {
		display: inline-flex;
		align-items: center;
		gap: var(--space-2);
		background: var(--accent-gradient);
		color: #fff;
		border: none;
		border-radius: var(--radius-pill, 999px);
		padding: var(--space-2) var(--space-5);
		font-family: inherit;
		font-weight: var(--fw-semibold);
		font-size: var(--fs-sm);
		cursor: pointer;
		box-shadow: 0 6px 20px -6px var(--primary-glow);
		&:hover:not(:disabled) {
			filter: brightness(1.06);
		}
		&:disabled {
			opacity: 0.55;
			cursor: not-allowed;
		}
	}
</style>
