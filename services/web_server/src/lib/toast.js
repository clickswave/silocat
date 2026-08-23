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

/**
 * Third argument is passed straight through to Sonner, which is where `action`
 * lives: `toast.success('Sharing turned on', 'Anyone with the link…', { action:
 * { label: 'Undo', onClick } })`. Kept as a separate parameter rather than
 * overloading the second, so the common two-string call stays the obvious one
 * and nobody has to remember whether the second slot is prose or config.
 */
const opts = (description, extra) => {
	const o = { ...(description ? { description } : null), ...(extra || null) };
	return Object.keys(o).length ? o : undefined;
};

export const toast = {
	success: (title, description, extra) => sonner.success(title, opts(description, extra)),
	error: (title, description, extra) => sonner.error(title, opts(description, extra)),
	warning: (title, description, extra) => sonner.warning(title, opts(description, extra)),
	info: (title, description, extra) => sonner.message(title, opts(description, extra)),
	/** Resolves/rejects a promise with a toast that swaps state in place. */
	promise: (p, msgs) => sonner.promise(p, msgs),
	loading: (title, description, extra) => sonner.loading(title, opts(description, extra)),
	dismiss: (id) => sonner.dismiss(id)
};

export default toast;
