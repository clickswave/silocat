<script>
	import Icon from '@iconify/svelte';
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
						<span>{fmt(d.loaded)}{d.total ? ` / ${fmt(d.total)}` : ''}</span>
						<span>{pct(d)}%</span>
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

<style>
	.dl-stack {
		position: fixed;
		right: 1rem;
		bottom: 1rem;
		z-index: 200;
		display: flex;
		flex-direction: column;
		gap: 0.6rem;
		width: min(340px, calc(100vw - 2rem));
	}
	.dl-card {
		background: var(--bg-card, #1b1b1f);
		border: 1px solid var(--border-default, #2e2e35);
		border-radius: var(--radius-md, 8px);
		padding: 0.7rem 0.8rem;
		box-shadow: 0 12px 32px rgba(0, 0, 0, 0.45);
		color: var(--text-primary, #e9e9ee);
	}
	.dl-top {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}
	:global(.dl-card .dl-ico) {
		flex-shrink: 0;
		color: var(--primary, #ff4655);
	}
	.dl-name {
		flex: 1;
		min-width: 0;
		font-size: 0.8rem;
		font-weight: 600;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.dl-x {
		flex-shrink: 0;
		background: transparent;
		border: none;
		color: var(--text-muted, #9a9aa3);
		cursor: pointer;
		display: flex;
		padding: 2px;
		border-radius: 4px;
	}
	.dl-x:hover {
		color: var(--text-primary, #fff);
		background: rgba(255, 255, 255, 0.08);
	}
	.dl-bar {
		height: 5px;
		background: var(--bg-input, #2a2a30);
		border-radius: 99px;
		overflow: hidden;
		margin: 0.5rem 0 0.35rem;
	}
	.dl-fill {
		height: 100%;
		background: var(--accent-gradient, linear-gradient(90deg, #ff4655, #ff8a93));
		border-radius: 99px;
		transition: width 0.2s ease;
	}
	.dl-bar.done .dl-fill {
		background: var(--success, #3ddc97);
	}
	.dl-bar.err .dl-fill {
		background: var(--text-muted, #9a9aa3);
	}
	.dl-meta {
		display: flex;
		justify-content: space-between;
		font-size: 0.7rem;
		color: var(--text-muted, #9a9aa3);
		font-variant-numeric: tabular-nums;
	}
	.dl-err {
		color: #ff6b6b;
	}
</style>
