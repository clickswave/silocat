<script>
	import Icon from '@iconify/svelte';

	let {
		icon = 'ri:inbox-2-line',
		title = 'Nothing here',
		line = undefined,
		children // optional action button(s)
	} = $props();
</script>

<div class="empty">
	<div class="glyph"><Icon {icon} width={22} /></div>
	<p class="title">{title}</p>
	{#if line}
		<p class="line">{line}</p>
	{/if}
	{#if children}
		<div class="action">{@render children()}</div>
	{/if}
</div>

<style lang="scss">
	.empty {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: var(--space-2);
		padding: var(--space-10) var(--space-4);
		text-align: center;
	}

	.glyph {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 44px;
		height: 44px;
		border: 1px solid var(--edge);
		border-radius: var(--radius-full);
		color: var(--ink-faint);
		margin-bottom: var(--space-1);
	}

	.title {
		font-size: var(--fs-body);
		font-weight: var(--fw-medium);
		color: var(--ink);
		margin: 0;
	}

	.line {
		font-size: var(--fs-sm);
		color: var(--ink-faint);
		margin: 0;
		max-width: 340px;
	}

	.action {
		margin-top: var(--space-3);
	}
</style>
