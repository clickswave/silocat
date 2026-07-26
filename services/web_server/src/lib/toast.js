/**
 * Themed toasts.
 *
 * The design replaces every native `alert()` / `window.prompt()` in the product
 * with a toast that reads: status glyph, title, optional faint second line.
 * Sonner stays as the transport (it already handles stacking, timers and
 * a11y live regions); the look is re-skinned from tokens in `global.scss`, so
 * light theme works instead of the old hardcoded-dark toasts.
 *
 * Usage:  toast.success('Link copied', 'Anyone with it can download the file.')
 */
import { toast as sonner } from 'svelte-sonner';

const opts = (description) => (description ? { description } : undefined);

export const toast = {
	success: (title, description) => sonner.success(title, opts(description)),
	error: (title, description) => sonner.error(title, opts(description)),
	warning: (title, description) => sonner.warning(title, opts(description)),
	info: (title, description) => sonner.message(title, opts(description)),
	/** Resolves/rejects a promise with a toast that swaps state in place. */
	promise: (p, msgs) => sonner.promise(p, msgs),
	loading: (title, description) => sonner.loading(title, opts(description)),
	dismiss: (id) => sonner.dismiss(id)
};

export default toast;
