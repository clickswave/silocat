/**
 * Trash retention, shared by the UI and matching the server sweep.
 *
 * The Trash screen promises "items stay here for 30 days, then delete
 * themselves" and shows a per-row countdown. WatchCat's `gc_expired_trash` job
 * is what actually enforces it (`WATCHCAT_TRASH_TTL_DAYS`, default 30), so this
 * constant has to stay in step with that default.
 */
export const TRASH_TTL_DAYS = 30;

/** Days at or below which the countdown switches to the warn colour. */
const SOON_DAYS = 5;

/**
 * Countdown label for a trashed item.
 * Returns `{ label, days, soon }`: `soon` drives the warn tint.
 */
export function autoDeleteIn(deletedOn, ttlDays = TRASH_TTL_DAYS) {
	if (!deletedOn) return { label: '-', days: null, soon: false };

	const deletedAt = new Date(deletedOn).getTime();
	if (!deletedAt) return { label: '-', days: null, soon: false };

	const dueAt = deletedAt + ttlDays * 86400_000;
	const msLeft = dueAt - Date.now();

	if (msLeft <= 0) return { label: 'due', days: 0, soon: true };

	const days = Math.ceil(msLeft / 86400_000);
	if (days <= 1) {
		const hours = Math.max(1, Math.ceil(msLeft / 3600_000));
		return { label: `${hours}h`, days: 0, soon: true };
	}

	return { label: `${days}d`, days, soon: days <= SOON_DAYS };
}
