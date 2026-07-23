<script>
	import Icon from '@iconify/svelte';
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { toast } from 'svelte-sonner';
	import { Button, Input } from '$lib/ui';

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
	}
</script>

<div class="view new-ticket">
	<header class="head">
		<a class="back" href="/home/support"><Icon icon="ri:arrow-left-line" width="16" /> Support</a>
		<h1 class="page-title">New ticket</h1>
		<p class="page-subtitle">Tell us what's going on, or share an idea. We'll reply by email and here.</p>
	</header>

	<form class="form" onsubmit={handleSubmit}>
		<div class="field">
			<span class="label">What's this about?</span>
			<div class="chips">
				{#each categories as c (c.id)}
					<button
						type="button"
						class="chip"
						class:active={category === c.id}
						onclick={() => (category = c.id)}
					>
						<Icon icon={c.icon} width="15" />
						<span>{c.label}</span>
					</button>
				{/each}
			</div>
		</div>

		<Input bind:value={email} type="email" label="Your email" icon="ri:mail-line" placeholder="you@example.com" hint="We'll reply to this address." autocomplete="email" />

		<Input bind:value={subject} label="Subject" icon="ri:text" placeholder="Brief summary" maxlength="120" />

		<div class="field">
			<label class="label" for="sup-message">Message</label>
			<textarea id="sup-message" rows="7" placeholder="Tell us what's going on…" bind:value={message}></textarea>
			<span class="hint">{message.length} characters</span>
		</div>

		<div class="actions">
			<Button variant="quiet" href="/home/support">Cancel</Button>
			<Button type="submit" loading={sending} disabled={!canSubmit}>Create ticket</Button>
		</div>
	</form>
</div>

<style lang="scss">
	.new-ticket {
		max-width: 640px;
	}
	.head {
		.back {
			display: inline-flex;
			align-items: center;
			gap: var(--space-1);
			color: var(--ink-faint);
			font-size: var(--fs-sm);
			margin-bottom: var(--space-3);
			&:hover {
				color: var(--ink);
			}
		}
	}

	.form {
		display: flex;
		flex-direction: column;
		gap: var(--space-5);
		background: var(--surface);
		border: 1px solid var(--edge);
		border-radius: var(--radius-md);
		padding: var(--space-6);
	}

	.field {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}
	.label {
		font-size: var(--fs-sm);
		font-weight: var(--fw-medium);
		color: var(--ink-mute);
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
			border-color var(--dur) var(--ease),
			color var(--dur) var(--ease),
			background var(--dur) var(--ease);

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

	textarea {
		width: 100%;
		background: var(--bg);
		border: 1px solid var(--edge);
		border-radius: var(--radius-sm);
		padding: 0.65rem 0.85rem;
		color: var(--ink);
		font-family: inherit;
		font-size: var(--fs-body);
		outline: none;
		resize: vertical;
		min-height: 130px;
		line-height: var(--lh-normal);
		transition:
			border-color var(--dur) var(--ease),
			box-shadow var(--dur) var(--ease);

		&::placeholder {
			color: var(--ink-faint);
		}
		&:focus {
			border-color: var(--accent);
			box-shadow: 0 0 0 3px var(--focus-ring);
		}
	}

	.actions {
		display: flex;
		justify-content: flex-end;
		align-items: center;
		gap: var(--space-2);
	}
</style>
