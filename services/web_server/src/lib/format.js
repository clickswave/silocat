/**
 * Formatting helpers shared by every screen.
 *
 * There were at least four copies of formatSize and two of the relative-time
 * helper, and they had diverged: some clamped the units index, some did not (so
 * formatSize(0.5) indexed sizes[-1] and returned "undefined"), and one relative
 * time renderer produced "2 hour ago" and "1 days ago".
 */

const UNITS = ['B', 'KB', 'MB', 'GB', 'TB', 'PB'];

/** `1.5 GB`. Handles 0, negatives, and sub-1-byte values without producing NaN. */
export function formatSize(bytes) {
	const n = Number(bytes);
	if (!Number.isFinite(n) || n <= 0) return '0 B';
	// Clamped at both ends: below 1 the log is negative and would index off the
	// front of the array, above TB it would run off the back.
	const i = Math.min(Math.max(Math.floor(Math.log(n) / Math.log(1024)), 0), UNITS.length - 1);
	return `${parseFloat((n / 1024 ** i).toFixed(1))} ${UNITS[i]}`;
}

/** `just now`, `5 minutes ago`, `3 days ago`, then an absolute date. */
export function relativeTime(value) {
	const then = new Date(value).getTime();
	if (!Number.isFinite(then)) return '';

	const seconds = (Date.now() - then) / 1000;
	// A clock skewed a little into the future should read as "just now", not as a
	// negative duration.
	if (seconds < 60) return 'just now';

	const plural = (n, word) => `${n} ${word}${n === 1 ? '' : 's'} ago`;

	if (seconds < 3600) return plural(Math.floor(seconds / 60), 'minute');
	if (seconds < 86400) return plural(Math.floor(seconds / 3600), 'hour');
	if (seconds < 86400 * 7) return plural(Math.floor(seconds / 86400), 'day');

	return new Date(value).toLocaleDateString(undefined, {
		month: 'short',
		day: 'numeric',
		year: new Date(value).getFullYear() === new Date().getFullYear() ? undefined : 'numeric'
	});
}

/** Compact form for dense rows: `now`, `5m`, `3h`, `2d`, then a date. */
export function shortTime(value) {
	const then = new Date(value).getTime();
	if (!Number.isFinite(then)) return '';
	const s = Math.max(0, (Date.now() - then) / 1000);
	if (s < 60) return 'now';
	if (s < 3600) return `${Math.floor(s / 60)}m`;
	if (s < 86400) return `${Math.floor(s / 3600)}h`;
	if (s < 86400 * 7) return `${Math.floor(s / 86400)}d`;
	return new Date(value).toLocaleDateString(undefined, { month: 'short', day: 'numeric' });
}
