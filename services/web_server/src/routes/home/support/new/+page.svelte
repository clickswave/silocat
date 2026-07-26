<script>
	import Icon from '$lib/ui/Icon.svelte';
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { toast } from '$lib/toast.js';
	

	let user = $derived($page.data.user || {});

	const categories = [
		{ id: 'help', label: 'Need help', icon: 'ri:question-line' },
		{ id: 'suggestion', label: 'Suggestion', icon: 'ri:lightbulb-line' },
		{ id: 'bug', label: 'Report a bug', icon: 'ri:bug-line' },
		{ id: 'account', label: 'Account / billing', icon: 'ri:bank-card-line' },
		{ id: 'other', label: 'Something else', icon: 'ri:chat-3-line' }
	];

	let category = $state('help');
	let subject = $state('');
	let message = $state('');
	let email = $state('');
	let sending = $state(false);

	$effect(() => {
		if (user?.email && !email) email = user.email;
	});

	let canSubmit = $derived(subject.trim().length > 2 && message.trim().length > 5);

	async function handleSubmit(e) {
		e.preventDefault();
		if (!canSubmit || sending) return;
		sending = true;
		try {
			const res = await fetch('/api/v1/user/support', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					category,
					subject: subject.trim(),
					message: message.trim(),
					email: email.trim()
				})
			});
			const d = await res.json();
			if (!res.ok) {
				toast.error(d.message || (d.errors && d.errors.join(', ')) || d.error || 'Could not send message');
			} else {
				toast.success('Your ticket has been created.');
				goto('/home/support');
			}
		} catch (err) {
			console.error(err);
			toast.error('Could not send message. Please try again.');
		} finally {
			sending = false;
		}
	}</script>

<div class="new-ticket">
	<a class="back" href="/home/support">← Support</a>

	<header class="head">
		<h1>New ticket</h1>
		<span class="sub">
			Tell us what's going on, or share an idea. We'll reply by email and here.
		</span>
	</header>

	<form class="form" onsubmit={handleSubmit}>
		<div class="f-field">
			<span class="label">What's this about?</span>
			<div class="chips">
				{#each categories as c (c.id)}
					<button
						type="button"
						class="chip"
						class:active={category === c.id}
						onclick={() => (category = c.id)}
					>
						<Icon name={c.icon} size={15} />
						<span>{c.label}</span>
					</button>
				{/each}
			</div>
		</div>

		<div class="f-field">
			<label for="sup-email">Your email</label>
			<input
				id="sup-email"
				type="email"
				bind:value={email}
				placeholder="you@example.com"
				autocomplete="email"
			/>
			<span class="hint">We'll reply to this address.</span>
		</div>

		<div class="f-field">
			<label for="sup-subject">Subject</label>
			<input
				id="sup-subject"
				type="text"
				bind:value={subject}
				placeholder="Brief summary"
				maxlength="120"
			/>
		</div>

		<div class="f-field">
			<label for="sup-message">Message</label>
			<textarea
				id="sup-message"
				rows="7"
				placeholder="Tell us what's going on…"
				bind:value={message}
			></textarea>
			<span class="hint">{message.length} characters</span>
		</div>

		<div class="actions">
			<a class="cancel" href="/home/support">Cancel</a>
			<button type="submit" class="primary" disabled={!canSubmit || sending}>
				{sending ? 'Creating…' : 'Create ticket'}
			</button>
		</div>
	</form>
</div>

<style lang="scss">
	.new-ticket {
		max-width: 640px;
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
		flex-direction: column;
		gap: var(--space-1);
		padding-inline: 0.125rem;

		h1 {
			margin: 0;
			font-size: var(--fs-h2);
			font-weight: var(--fw-black);
			letter-spacing: var(--tracking-tight);
			line-height: var(--lh-tight);
		}
	}

	.sub {
		font-size: var(--fs-sm);
		color: var(--ink-faint);
	}

	.form {
		display: flex;
		flex-direction: column;
		gap: 1.25rem;
		background: var(--surface);
		border: 1px solid var(--edge);
		border-radius: var(--radius-md);
		padding: 1.25rem;
	}

	.f-field {
		display: flex;
		flex-direction: column;
		gap: 0.375rem;

		label,
		.label {
			font-size: var(--fs-xs);
			color: var(--ink-mute);
		}

		input,
		textarea {
			height: 36px;
			padding: 0 0.625rem;
			border-radius: var(--radius-sm);
			background: var(--bg);
			border: 1px solid var(--edge);
			color: var(--ink);
			font-family: var(--font-sans);
			font-size: 0.875rem;
			outline: none;
			transition:
				border-color var(--dur-fast) var(--ease),
				box-shadow var(--dur-fast) var(--ease);

			&::placeholder {
				color: var(--ink-faint);
			}
			&:focus {
				border-color: var(--accent);
				box-shadow: 0 0 0 3px var(--focus-ring);
			}
		}

		textarea {
			height: auto;
			min-height: 130px;
			padding: 0.5rem 0.625rem;
			resize: vertical;
			line-height: var(--lh-normal);
		}
	}

	.hint {
		font-size: var(--fs-xs);
		color: var(--ink-faint);
	}

	.chips {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-2);
	}

	.chip {
		display: inline-flex;
		align-items: center;
		gap: var(--space-2);
		background: transparent;
		border: 1px solid var(--edge);
		color: var(--ink-mute);
		font-family: inherit;
		font-size: var(--fs-sm);
		font-weight: var(--fw-medium);
		padding: 0.4rem 0.75rem;
		border-radius: var(--radius-full);
		cursor: pointer;
		transition:
			border-color var(--dur-fast) var(--ease),
			color var(--dur-fast) var(--ease),
			background var(--dur-fast) var(--ease);

		&:hover {
			color: var(--ink);
			border-color: var(--edge-strong);
		}
		&.active {
			background: var(--accent-soft);
			border-color: transparent;
			color: var(--accent);
		}
	}

	.actions {
		display: flex;
		justify-content: flex-end;
		align-items: center;
		gap: var(--space-2);
	}

	.cancel {
		height: 36px;
		display: inline-flex;
		align-items: center;
		padding-inline: 0.875rem;
		border-radius: var(--radius-md);
		font-size: var(--fs-sm);
		color: var(--ink-mute);
		text-decoration: none;
		transition:
			background var(--dur-fast) var(--ease),
			color var(--dur-fast) var(--ease);

		&:hover {
			background: var(--tint-soft);
			color: var(--ink);
		}
	}

	.primary {
		height: 36px;
		padding-inline: 1rem;
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
			opacity: 0.55;
			cursor: not-allowed;
		}
	}
</style>
