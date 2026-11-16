<script>
	// Shown wherever an upload or a restore would start, while the account holds
	// as much as its plan allows or more. It shares the storage query with the
	// sidebar meter, so it clears itself once usage drops under the plan.
	import { browser } from '$app/environment';
	import { createQuery } from '@tanstack/svelte-query';
	import Icon from '$lib/ui/Icon.svelte';
	import { FrontendClient } from '$lib/frontendClient.js';
	import { qk } from '$lib/queryKeys.js';
	import { isOverQuota, OVER_QUOTA_NOTICE } from '$lib/storageQuota.js';

	const storageQuery = createQuery(() => ({
		queryKey: qk.storage,
		queryFn: async () => {
			try {
				const { data } = await FrontendClient.get('/api/v1/sanctum/user/storage');
				return data?.success || { total: 0, used: 0, free: 0 };
			} catch {
				return { total: 0, used: 0, free: 0 };
			}
		},
		enabled: browser
	}));

	let show = $derived(isOverQuota(storageQuery.data));
</script>

{#if show}
	<div class="over-quota" role="status">
		<Icon name="alert" size={15} />
		<span>{OVER_QUOTA_NOTICE}</span>
		<a href="/home/billing">See plans</a>
	</div>
{/if}

<style lang="scss">
	.over-quota {
		display: flex;
		align-items: flex-start;
		gap: var(--space-2);
		padding: 0.625rem 0.875rem;
		border-radius: var(--radius-md);
		background: var(--warn-soft);
		border: 1px solid var(--edge);
		color: var(--warn);
		font-size: var(--fs-sm);
		flex: 0 0 auto;

		span {
			flex: 1;
			color: var(--ink-mute);
		}

		a {
			color: var(--warn);
			font-weight: var(--fw-medium);
			white-space: nowrap;
		}
	}
</style>
