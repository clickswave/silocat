/**
 * Stop a navigation from silently destroying a transfer in progress.
 *
 * The core action of this product is moving gigabytes through the browser, and
 * until now closing the tab, hitting reload, or clicking a link in the sidebar
 * threw away however much of that had completed, with no warning at all. There
 * was no beforeunload handler anywhere in the app.
 *
 * Two guards, because there are two ways to leave:
 *   * `beforeunload` for closing, reloading and off-site navigation. Browsers
 *     only honour it if something has actually claimed a transfer, and they show
 *     their own generic wording; the string we pass is ignored by every current
 *     browser but is still required to trigger the prompt in some.
 *   * SvelteKit's `beforeNavigate` for in-app navigation, which never triggers
 *     `beforeunload` and is the far more likely accident: the rail is one click
 *     away from the upload modal.
 *
 * Counted rather than boolean, so concurrent uploads and downloads do not
 * un-guard each other when the first one finishes.
 */
import { beforeNavigate } from '$app/navigation';
import { forcesFullLoad } from './analytics.js';

const DEFAULT_MESSAGE = 'A transfer is still running. Leaving this page will cancel it.';

let active = 0;
let listening = false;
// The prompt of the guard installed last, for confirmFullLoad.
let message = DEFAULT_MESSAGE;
// Set when the visitor has just agreed in confirmFullLoad to a full page load.
// The beforeunload that load fires is let through once, without asking again.
let leaveAgreed = false;

function onBeforeUnload(e) {
	if (active <= 0) return;
	if (leaveAgreed) {
		leaveAgreed = false;
		return;
	}
	e.preventDefault();
	e.returnValue = ''; // required by older browsers to trigger the prompt
	return '';
}

function sync() {
	if (typeof window === 'undefined') return;
	if (active > 0 && !listening) {
		window.addEventListener('beforeunload', onBeforeUnload);
		listening = true;
	} else if (active <= 0 && listening) {
		window.removeEventListener('beforeunload', onBeforeUnload);
		listening = false;
	}
}

/**
 * Mark a transfer as running. Returns the release function; call it in a
 * `finally` so a thrown error cannot leave the guard stuck on.
 */
export function holdTransfer() {
	active++;
	leaveAgreed = false;
	sync();
	let released = false;
	return () => {
		if (released) return;
		released = true;
		active = Math.max(0, active - 1);
		sync();
	};
}

export function transfersActive() {
	return active > 0;
}

/**
 * Ask before a full page load that would end a running transfer. The root
 * layout makes one in place of an in-app navigation onto or off a drop, share
 * or request page while GA runs (see $lib/analytics.js), and the guard below
 * leaves those to this. True when nothing is running or the visitor agreed to
 * leave, and then beforeunload does not ask a second time. On iOS Safari, which
 * never shows a beforeunload prompt, this is the only one.
 */
export function confirmFullLoad() {
	if (active <= 0) return true;
	if (!confirm(message)) return false;
	leaveAgreed = true;
	return true;
}

/**
 * Install the in-app navigation guard. Call once, from a component that lives
 * for the whole session (the /home layout and the public pages that transfer).
 */
export function guardNavigation(text = DEFAULT_MESSAGE) {
	message = text;
	beforeNavigate((nav) => {
		if (active <= 0) return;

		// A full-page unload is already covered by beforeunload; double-prompting
		// is worse than not prompting.
		if (nav.willUnload) return;

		// The root layout turns this move into a full page load and asks through
		// confirmFullLoad. Asking here as well would ask twice, and a Cancel here
		// would not stop that load.
		if (nav.to && forcesFullLoad(nav.from?.url, nav.to.url)) return;

		// Staying on the same route does not tear the page down, so the transfer
		// survives and there is nothing to warn about. This matters because the
		// Files page keeps the current folder in the URL: without this check,
		// opening a folder mid-upload prompts "this will cancel it", which is both
		// alarming and false. Query-string-only changes are the same page.
		if (nav.to && nav.from && nav.to.url.pathname === nav.from.url.pathname) return;

		if (!confirm(text)) nav.cancel();
	});
}
