<script>
	// Shown under a download, to the person who received the file. Most of
	// silo.cat's traffic is these recipients, and the page used to end at
	// "Shared via Silocat". Links on a paying sender's pages leave it out; that
	// is one of the things the paid plans buy.
	//
	// Every link carries a `ref`, which $lib/source.js keeps until signup, so
	// the console can count the accounts this block earns.
	import Icon from '$lib/ui/Icon.svelte';
	import { trackCta } from '$lib/track.js';

	/** @type {{ surface: 'drop' | 'share', done?: boolean }} */
	let { surface, done = false } = $props();
</script>

<aside class="sy" class:done aria-label="Send files with Silocat">
	<div class="sy-text">
		<p class="sy-title">{done ? 'Got it. Need to send something back?' : 'Send files like this, free'}</p>
		<p class="sy-body">
			Drop a file and get a link. Up to 20&nbsp;GB, encrypted in your browser if you like, and no
			account needed.
		</p>
	</div>
	<div class="sy-actions">
		<a
			class="sy-primary"
			href="/?ref=dl-{surface}-send"
			onclick={() => trackCta('download_page', { target: 'send', surface, done })}
		>
			<Icon icon="ri:upload-cloud-2-line" width="15" /> Send a file
		</a>
		<a
			class="sy-link"
			href="/pricing?ref=dl-{surface}-request"
			onclick={() => trackCta('download_page', { target: 'request', surface, done })}
		>
			Collect files from clients
		</a>
	</div>
</aside>

<style>
	.sy {
		width: 100%;
		max-width: var(--sy-max, 460px);
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-4);
		padding: var(--space-4) var(--space-5);
		border: 1px dashed var(--edge);
		border-radius: var(--radius-lg);
	}
	.sy.done {
		border-style: solid;
		background: var(--surface);
	}
	.sy-text {
		min-width: 0;
	}
	.sy-title {
		margin: 0;
		font-size: var(--fs-sm);
		font-weight: 600;
		color: var(--ink);
	}
	.sy-body {
		margin: var(--space-1) 0 0;
		font-size: var(--fs-xs);
		color: var(--ink-faint);
		line-height: 1.5;
	}
	.sy-actions {
		display: flex;
		flex-direction: column;
		align-items: flex-end;
		gap: var(--space-2);
		flex: none;
	}
	.sy-primary {
		display: inline-flex;
		align-items: center;
		gap: var(--space-1);
		padding: var(--space-2) var(--space-3);
		border: 1px solid var(--edge);
		border-radius: var(--radius-md);
		font-size: var(--fs-sm);
		color: var(--ink);
		text-decoration: none;
		white-space: nowrap;
	}
	.sy-primary:hover {
		border-color: var(--accent);
		color: var(--accent);
	}
	.sy-link {
		font-size: var(--fs-xs);
		color: var(--ink-faint);
		white-space: nowrap;
	}
	.sy-link:hover {
		color: var(--ink);
	}
	@media (max-width: 480px) {
		.sy {
			flex-direction: column;
			align-items: flex-start;
		}
		.sy-actions {
			flex-direction: row;
			align-items: center;
			gap: var(--space-4);
		}
	}
</style>
