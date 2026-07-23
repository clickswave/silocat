<script>
	let { src = undefined, name = '', size = 32 } = $props();

	let initial = $derived((name || '?').trim().charAt(0).toUpperCase());
</script>

{#if src}
	<img class="avatar" {src} alt={name} width={size} height={size} style="width:{size}px;height:{size}px" />
{:else}
	<span class="avatar fallback" style="width:{size}px;height:{size}px;font-size:{Math.round(size * 0.42)}px">
		{initial}
	</span>
{/if}

<style lang="scss">
	.avatar {
		border-radius: var(--radius-full);
		object-fit: cover;
		flex-shrink: 0;
	}
	.fallback {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		background: var(--tint-softer);
		border: 1px solid var(--edge);
		color: var(--ink-mute);
		font-weight: var(--fw-semibold);
		user-select: none;
	}
</style>
