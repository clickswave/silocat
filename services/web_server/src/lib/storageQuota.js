// Whether an account has no room left under its plan, from the storage stats
// the API returns.
//
// An account that holds as much as its plan allows or more, which is what a
// lapsed subscription leaves behind, is read-only: the server refuses uploads
// and restores until usage is under the plan, and deletes nothing.

export const OVER_QUOTA_NOTICE =
	'Your plan has lapsed or is full, so uploads and restores are paused. Your files stay readable and shareable, and nothing is deleted. Uploads and restores resume once your usage is under your plan.';

/** @param {{ total?: number, used?: number, free?: number, over_quota?: boolean } | null | undefined} stats */
export function isOverQuota(stats) {
	if (!stats) return false;
	if (typeof stats.over_quota === 'boolean') return stats.over_quota;
	// A server from before the flag: work it out the same way. A zero total
	// means the stats have not loaded or the call failed, not an empty plan.
	const total = Number(stats.total) || 0;
	const used = Number(stats.used) || 0;
	return total > 0 && used >= total;
}
