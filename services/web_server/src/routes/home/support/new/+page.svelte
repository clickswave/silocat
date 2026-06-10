<script>
	import Icon from '@iconify/svelte';
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { toast } from 'svelte-sonner';

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

<div class="support-page">
	<header class="page-header">
		<a class="back" href="/home/support"><Icon icon="ri:arrow-left-line" width="18" /> Support</a>
		<h1>New ticket</h1>
		<p class="subtitle">Tell us what is going on, or share an idea. We will reply by email and here.</p>
	</header>

	<form class="card form-card" onsubmit={handleSubmit}>
		<div class="field">
			<span class="field-label">What is this about?</span>
			<div class="cat-chips">
				{#each categories as c}
					<button
						type="button"
						class="cat-chip {category === c.id ? 'active' : ''}"
						onclick={() => (category = c.id)}
					>
						<Icon icon={c.icon} width="16" />
						<span>{c.label}</span>
					</button>
				{/each}
			</div>
		</div>

		<div class="field">
			<label class="field-label" for="sup-email">Your email</label>
			<input id="sup-email" type="email" placeholder="you@example.com" bind:value={email} autocomplete="email" />
			<span class="field-hint">We will reply to this address.</span>
		</div>

		<div class="field">
			<label class="field-label" for="sup-subject">Subject</label>
			<input id="sup-subject" type="text" placeholder="Brief summary" bind:value={subject} maxlength="120" />
		</div>

		<div class="field">
			<label class="field-label" for="sup-message">Message</label>
			<textarea id="sup-message" rows="7" placeholder="Tell us what is going on…" bind:value={message}></textarea>
			<span class="field-hint">{message.length} characters</span>
		</div>

		<div class="form-actions">
			<a class="btn-ghost" href="/home/support">Cancel</a>
			<button type="submit" class="submit-btn" disabled={!canSubmit || sending}>
				<Icon icon={sending ? 'svg-spinners:ring-resize' : 'ri:send-plane-2-line'} width="18" />
				{sending ? 'Sending…' : 'Create ticket'}
			</button>
		</div>
	</form>
</div>

<style lang="scss">
	.support-page {
		width: 100%;
		max-width: 760px;
		color: var(--text-primary);
		display: flex;
		flex-direction: column;
		gap: var(--space-6);
	}
	.page-header {
		.back {
			display: inline-flex;
			align-items: center;
			gap: var(--space-1);
			color: var(--text-muted);
			font-size: var(--fs-sm);
			margin-bottom: var(--space-3);
			&:hover {
				color: var(--text-primary);
			}
		}
		h1 {
			font-size: var(--fs-h3);
			font-weight: var(--fw-semibold);
			margin: 0 0 var(--space-1);
		}
		.subtitle {
			color: var(--text-muted);
			font-size: var(--fs-sm);
			margin: 0;
			max-width: 60ch;
		}
	}
	.card {
		background: var(--bg-card);
		border: 1px solid var(--border-default);
		border-radius: var(--radius-md);
		box-shadow: var(--shadow-card);
	}
	.form-card {
		padding: var(--space-6);
		display: flex;
		flex-direction: column;
		gap: var(--space-5);
	}
	.field {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}
	.field-label {
		font-size: var(--fs-sm);
		font-weight: var(--fw-medium);
		color: var(--text-secondary);
	}
	.field-hint {
		font-size: var(--fs-xs);
		color: var(--text-muted);
	}
	.cat-chips {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-2);
	}
	.cat-chip {
		display: inline-flex;
		align-items: center;
		gap: var(--space-2);
		background: var(--tint-soft);
		border: 1px solid var(--border-default);
		color: var(--text-secondary);
		font-family: inherit;
		font-size: var(--fs-sm);
		font-weight: var(--fw-medium);
		padding: var(--space-2) var(--space-3);
		border-radius: var(--radius-pill, 999px);
		cursor: pointer;
		transition: border-color var(--dur) var(--ease), color var(--dur) var(--ease);
		&:hover {
			color: var(--text-primary);
		}
		&.active {
			border-color: var(--primary);
			color: var(--primary);
		}
	}
	input,
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
		transition: border-color var(--dur) var(--ease), box-shadow var(--dur) var(--ease);
		&::placeholder {
			color: var(--text-muted);
		}
		&:focus {
			border-color: var(--primary);
			box-shadow: 0 0 0 3px var(--primary-glow);
		}
	}
	textarea {
		resize: vertical;
		min-height: 120px;
		line-height: var(--lh-normal, 1.5);
	}
	.form-actions {
		display: flex;
		justify-content: flex-end;
		align-items: center;
		gap: var(--space-3);
	}
	.btn-ghost {
		color: var(--text-secondary);
		font-size: var(--fs-sm);
		font-weight: var(--fw-medium);
		padding: var(--space-2) var(--space-3);
		border-radius: var(--radius-pill, 999px);
		&:hover {
			color: var(--text-primary);
		}
	}
	.submit-btn {
		display: inline-flex;
		align-items: center;
		gap: var(--space-2);
		background: var(--accent-gradient);
		color: #fff;
		border: none;
		border-radius: var(--radius-pill, 999px);
		padding: var(--space-3) var(--space-6);
		font-family: inherit;
		font-weight: var(--fw-semibold);
		font-size: var(--fs-body);
		cursor: pointer;
		box-shadow: 0 6px 20px -6px var(--primary-glow);
		transition: filter var(--dur) var(--ease);
		&:hover:not(:disabled) {
			filter: brightness(1.06);
		}
		&:disabled {
			opacity: 0.55;
			cursor: not-allowed;
		}
	}
</style>
