<script>
	import Icon from '$lib/ui/Icon.svelte';
	import { fly } from 'svelte/transition';
	import { flip } from 'svelte/animate';
	import { downloads, cancelDownload, dismissDownload } from '$lib/download.js';

	function pct(d) {
		if (d.status === 'done') return 100;
		if (!d.total) return 0;
		return Math.min(100, Math.round((d.loaded / d.total) * 100));
	}
	function fmt(bytes) {
		if (!bytes) return '0 B';
		const k = 1024,
			sizes = ['B', 'KB', 'MB', 'GB'];
		const i = Math.min(Math.floor(Math.log(bytes) / Math.log(k)), sizes.length - 1);
		return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
	}
</script>

{#if $downloads.length}
	<div class="dl-stack" aria-live="polite">
		{#each $downloads as d (d.id)}
			<div class="dl-card" animate:flip={{ duration: 200 }} transition:fly={{ y: 16, duration: 200 }}>
				<div class="dl-top">
					<Icon
						class="dl-ico"
						icon={d.status === 'active'
							? 'svg-spinners:ring-resize'
							: d.status === 'done'
								? 'ri:checkbox-circle-fill'
								: d.status === 'cancelled'
									? 'ri:close-circle-fill'
									: 'ri:error-warning-fill'}
						width="18"
					/>
					<span class="dl-name" title={d.name}>{d.name}</span>
					{#if d.status === 'active'}
						<button class="dl-x" title="Cancel" aria-label="Cancel download" onclick={() => cancelDownload(d.id)}>
							<Icon icon="ri:close-line" width="16" />
						</button>
					{:else}
						<button class="dl-x" title="Dismiss" aria-label="Dismiss" onclick={() => dismissDownload(d.id)}>
							<Icon icon="ri:close-line" width="16" />
						</button>
					{/if}
				</div>

				<div class="dl-bar" class:done={d.status === 'done'} class:err={d.status === 'error' || d.status === 'cancelled'}>
					<div class="dl-fill" style="width:{pct(d)}%"></div>
				</div>

				<div class="dl-meta">
					{#if d.status === 'active'}
						{#if d.phase && !d.loaded}
							<span class="dl-phase">{d.phase}</span><span></span>
						{:else}
							<span>{fmt(d.loaded)}{d.total ? ` / ${fmt(d.total)}` : ''}</span>
							<span>{pct(d)}%</span>
						{/if}
					{:else if d.status === 'done'}
						<span>Downloaded</span><span>{fmt(d.total)}</span>
					{:else if d.status === 'cancelled'}
						<span>Cancelled</span><span></span>
					{:else}
						<span class="dl-err">{d.error || 'Failed'}</span><span></span>
					{/if}
				</div>
			</div>
		{/each}
	</div>
{/if}

<style lang="scss">
	.dl-stack {
		position: fixed;
		right: 1rem;
		bottom: 1rem;
		z-index: 200;
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		width: min(340px, calc(100vw - 2rem));
	}
	.dl-card {
		background: var(--raised);
		border: 1px solid var(--edge);
		border-radius: var(--radius-md);
		padding: var(--space-3);
		box-shadow: var(--shadow-overlay);
		color: var(--ink);
	}
	.dl-top {
		display: flex;
		align-items: center;
		gap: var(--space-2);
	}
	:global(.dl-card .dl-ico) {
		flex-shrink: 0;
		color: var(--ink-mute);
	}
	.dl-name {
		flex: 1;
		min-width: 0;
		font-size: var(--fs-sm);
		font-weight: var(--fw-medium);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.dl-x {
		flex-shrink: 0;
		background: transparent;
		border: none;
		color: var(--ink-faint);
		cursor: pointer;
		display: flex;
		padding: 2px;
		border-radius: var(--radius-sm);

		&:hover {
			color: var(--ink);
			background: var(--tint-soft);
		}
	}
	.dl-bar {
		height: 4px;
		background: var(--tint-softer);
		border-radius: var(--radius-full);
		overflow: hidden;
		margin: var(--space-2) 0 var(--space-1);
	}
	.dl-fill {
		height: 100%;
		background: var(--accent);
		border-radius: var(--radius-full);
		transition: width 0.2s ease;
	}
	.dl-bar.done .dl-fill {
		background: var(--ok);
	}
	.dl-bar.err .dl-fill {
		background: var(--ink-faint);
	}
	.dl-meta {
		display: flex;
		justify-content: space-between;
		font-family: var(--font-mono);
		font-size: var(--fs-xs);
		color: var(--ink-faint);
		font-variant-numeric: tabular-nums;
	}
	.dl-err {
		color: var(--danger);
	}
	.dl-phase {
		color: var(--ink-mute);
		font-weight: var(--fw-medium);
	}
</style>
