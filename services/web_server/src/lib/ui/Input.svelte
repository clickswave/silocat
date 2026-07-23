<script>
	import Icon from '@iconify/svelte';

	let {
		value = $bindable(''),
		type = 'text',
		label = undefined,
		icon = undefined,
		error = undefined,
		hint = undefined,
		id = undefined,
		mono = false,
		children, // trailing controls (buttons) rendered inside the field
		...rest
	} = $props();

	const uid = id || `in-${Math.random().toString(36).slice(2, 8)}`;
</script>

<div class="input-block">
	{#if label}
		<label class="label" for={uid}>{label}</label>
	{/if}
	<div class="wrap" class:has-error={error}>
		{#if icon}
			<span class="lead"><Icon {icon} width={15} /></span>
		{/if}
		<input {id} class="input" class:mono {type} bind:value {...rest} />
		{#if children}
			<span class="trail">{@render children()}</span>
		{/if}
	</div>
	{#if error}
		<p class="note error">{error}</p>
	{:else if hint}
		<p class="note">{hint}</p>
	{/if}
</div>

<style lang="scss">
	.input-block {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		width: 100%;
	}

	.label {
		font-size: var(--fs-sm);
		font-weight: var(--fw-medium);
		color: var(--ink-mute);
	}

	.wrap {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		background: var(--bg);
		border: 1px solid var(--edge);
		border-radius: var(--radius-sm);
		padding-inline: 0.85rem;
		transition:
			border-color var(--dur) var(--ease),
			box-shadow var(--dur) var(--ease);

		&:focus-within {
			border-color: var(--accent);
			box-shadow: 0 0 0 3px var(--focus-ring);
		}
		&.has-error {
			border-color: var(--danger);
			&:focus-within {
				box-shadow: 0 0 0 3px var(--danger-soft);
			}
		}
	}

	.lead {
		display: flex;
		color: var(--ink-faint);
		flex-shrink: 0;
	}

	.trail {
		display: flex;
		align-items: center;
		gap: var(--space-1);
		flex-shrink: 0;
	}

	.input {
		flex: 1;
		min-width: 0;
		background: none;
		border: none;
		outline: none;
		color: var(--ink);
		font-family: inherit;
		font-size: var(--fs-body);
		padding: 0.65rem 0;

		&.mono {
			font-family: var(--font-mono);
		}
		&::placeholder {
			color: var(--ink-faint);
		}
	}

	.note {
		font-size: var(--fs-xs);
		color: var(--ink-faint);
		margin: 0;

		&.error {
			color: var(--danger);
		}
	}
</style>
