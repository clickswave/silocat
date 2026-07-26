<script>
	import Icon from '$lib/ui/Icon.svelte';
	import { page } from '$app/stores';
	import { toast } from '$lib/toast.js';
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
	/** `12 Jul, 14:02`: short and mono, matching every other timestamp. */
	function fmtShort(iso) {
		if (!iso) return '';
		const d = new Date(iso);
		return `${d.toLocaleDateString(undefined, { day: 'numeric', month: 'short' })}, ${d.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit', hour12: false })}`;
	}

	/** The opening message plus replies, as one ordered conversation. */
	let messages = $derived(
		ticket
			? [
					{
						id: 'open',
						role: 'user',
						body: ticket.message,
						at: ticket.created_at
					},
					...replies.map((r) => ({
						id: r.id,
						role: r.author_role === 'admin' ? 'admin' : 'user',
						body: r.body,
						at: r.created_at
					}))
				]
			: []
	);
</script>

<div class="thread">
	{#if loading}
		<div class="state"><Icon name="spinner" size={26} /></div>
	{:else if notFound}
		<div class="state empty">
			<Icon name="alert" size={34} stroke={1.4} />
			<span class="state-title">Ticket not found</span>
			<a class="back" href="/home/support">← All tickets</a>
		</div>
	{:else}
		<a class="back" href="/home/support">← All tickets</a>

		<header class="head">
			<div class="head-text">
				<div class="head-top">
					<span class="cat">{catLabel(ticket.category)}</span>
					<span class="status {ticket.status === 'open' ? 'open' : 'done'}">
						{ticket.status === 'open' ? 'Open' : 'Resolved'}
					</span>
				</div>
				<h1>{ticket.subject}</h1>
				<span class="opened">Opened {fmt(ticket.created_at)}</span>
			</div>
			<button
				type="button"
				class="status-btn"
				disabled={statusBusy}
				onclick={() => setStatus(ticket.status === 'open' ? 'closed' : 'open')}
			>
				{ticket.status === 'open' ? 'Close ticket' : 'Reopen'}
			</button>
		</header>

		<div class="conversation">
			{#each messages as m (m.id)}
				<div class="msg">
					<div class="msg-head">
						{#if m.role === 'admin'}
							<span class="who team">
								<Icon name="shield-check" size={13} stroke={1.8} />
								Silocat Team
							</span>
						{:else}
							<span class="who">You</span>
						{/if}
						<span class="at">{fmtShort(m.at)}</span>
					</div>
					<div class="bubble" class:team={m.role === 'admin'}>{m.body}</div>
				</div>
			{/each}
		</div>

		<form class="reply" onsubmit={sendReply}>
			<textarea
				rows="3"
				placeholder={ticket.status === 'closed' ? 'Reply to reopen this ticket…' : 'Write a reply…'}
				bind:value={body}
			></textarea>
			<div class="reply-foot">
				{#if ticket.status === 'closed'}
					<span class="hint">Sending a reply reopens this ticket.</span>
				{/if}
				<button type="submit" class="send" disabled={body.trim().length < 2 || sending}>
					{sending ? 'Sending…' : 'Send'}
					<Icon name={sending ? 'spinner' : 'send'} size={14} stroke={1.8} />
				</button>
			</div>
		</form>
	{/if}
</div>

<style lang="scss">
	.thread {
		max-width: 720px;
		display: flex;
		flex-direction: column;
		gap: 1.25rem;
		padding-bottom: var(--space-6);
	}

	.back {
		align-self: flex-start;
		font-size: var(--fs-sm);
		color: var(--ink-mute);
		text-decoration: none;
		padding-top: var(--space-2);
		transition: color var(--dur-fast) var(--ease);

		&:hover {
			color: var(--ink);
		}
	}

	.head {
		display: flex;
		align-items: flex-start;
		justify-content: space-between;
		gap: var(--space-4);
		padding-inline: 0.125rem;
	}

	.head-text {
		display: flex;
		flex-direction: column;
		gap: 0.375rem;
		min-width: 0;

		h1 {
			margin: 0;
			font-size: 1.25rem;
			font-weight: var(--fw-black);
			letter-spacing: var(--tracking-tight);
			line-height: 1.2;
		}
	}

	.head-top {
		display: flex;
		align-items: center;
		gap: var(--space-2);
	}

	.cat {
		font-family: var(--font-mono);
		font-size: 0.6875rem;
		color: var(--ink-faint);
		text-transform: uppercase;
		letter-spacing: 0.08em;
	}

	.status {
		display: inline-flex;
		align-items: center;
		height: 18px;
		padding-inline: 0.375rem;
		border-radius: var(--radius-sm);
		font-size: 0.6875rem;
		font-weight: var(--fw-medium);

		&.open {
			background: var(--ok-soft);
			color: var(--ok);
		}
		&.done {
			background: var(--tint-softer);
			color: var(--ink-mute);
		}
	}

	.opened {
		font-family: var(--font-mono);
		font-size: var(--fs-xs);
		color: var(--ink-faint);
	}

	.status-btn {
		flex: 0 0 auto;
		height: 32px;
		padding-inline: 0.75rem;
		border-radius: var(--radius-md);
		border: 1px solid var(--edge);
		background: none;
		font: inherit;
		font-size: var(--fs-sm);
		font-weight: var(--fw-medium);
		color: var(--ink-mute);
		cursor: pointer;
		transition:
			background var(--dur-fast) var(--ease),
			color var(--dur-fast) var(--ease);

		&:hover:not(:disabled) {
			background: var(--tint-soft);
			color: var(--ink);
		}
		&:disabled {
			opacity: 0.55;
			cursor: not-allowed;
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
		gap: 0.375rem;
	}

	.msg-head {
		display: flex;
		align-items: baseline;
		gap: var(--space-2);
	}

	.who {
		font-size: var(--fs-sm);
		font-weight: var(--fw-semibold);

		/* The team badge is the one place accent appears in the thread, so a
		   reply from support is unmistakable at a glance. */
		&.team {
			display: inline-flex;
			align-items: center;
			gap: 0.3125rem;
			color: var(--ink);

			:global(svg) {
				color: var(--accent);
			}
		}
	}

	.at {
		font-family: var(--font-mono);
		font-size: 0.6875rem;
		color: var(--ink-faint);
	}

	.bubble {
		padding: 0.75rem 0.875rem;
		border-radius: var(--radius-md);
		background: var(--surface);
		border: 1px solid var(--edge);
		font-size: 0.875rem;
		color: var(--ink);
		line-height: var(--lh-normal);
		white-space: pre-wrap;
		word-break: break-word;

		&.team {
			background: var(--accent-soft);
		}
	}

	.reply {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		padding: 1rem;
		border: 1px solid var(--edge);
		border-radius: var(--radius-md);
		background: var(--surface);

		textarea {
			padding: 0.25rem 0;
			border: 0;
			background: none;
			color: var(--ink);
			font-family: var(--font-sans);
			font-size: 0.875rem;
			outline: none;
			resize: vertical;
			min-height: 60px;
			line-height: var(--lh-normal);

			&::placeholder {
				color: var(--ink-faint);
			}
		}
	}

	.reply-foot {
		display: flex;
		align-items: center;
		justify-content: flex-end;
		gap: var(--space-3);
	}

	.hint {
		font-size: var(--fs-xs);
		color: var(--ink-faint);
		margin-right: auto;
	}

	.send {
		display: flex;
		align-items: center;
		gap: 0.4375rem;
		height: 34px;
		padding-inline: 0.875rem;
		border: 0;
		border-radius: var(--radius-md);
		background: var(--accent);
		color: #fff;
		font: inherit;
		font-size: var(--fs-sm);
		font-weight: var(--fw-medium);
		cursor: pointer;
		transition: background var(--dur-fast) var(--ease);

		&:hover:not(:disabled) {
			background: var(--accent-hover);
		}
		&:disabled {
			opacity: 0.5;
			cursor: not-allowed;
		}
	}

	.state {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.875rem;
		padding: var(--space-10) 0;
		color: var(--ink-faint);
	}

	.state-title {
		font-size: var(--fs-lg);
		font-weight: var(--fw-medium);
		color: var(--ink);
	}
</style>
