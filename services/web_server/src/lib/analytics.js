// Where Google Analytics runs on silo.cat, and where it must not.
//
// Three kinds of silo.cat URL are credentials in themselves:
//   /<id>         a drop. For an unencrypted public drop the id is all it
//                 takes to download the file.
//   /s/<token>    a share link.
//   /req/<token>  a file request: the token lets anyone upload into the
//                 owner's request.
// GA does not load on these pages at all. Handing gtag.js another page_location
// there is not enough: it also reads location.href and document.referrer for
// itself, in its tag diagnostics and in the page view it sends when a page
// comes back from the back-forward cache, and no parameter of ours reaches
// either.
//
// While GA runs on the site, moving onto or off one of these pages loads the
// next page in full. A page where GA runs therefore never has a link page in
// its own history, where gtag.js would report it as a History API change, and
// GA starts afresh on the page after a link page. That page is told only
// silo.cat's origin as its referrer, because every page goes out with
// Referrer-Policy: strict-origin ($lib/server/securityHeaders.js).
//
// SvelteKit's beforeNavigate is not enough for that on its own: the router
// skips every beforeNavigate callback while another navigation is still
// loading, and for the target of a redirect. So a click on a link to one of
// these pages is turned into a full load before the router sees it, and no
// History API entry for one of them is ever written on a page where GA runs:
// startAnalytics wraps pushState and replaceState to load the page in full
// instead.
//
// /home/files keeps the open folder's id and the names of the folders above it
// in its query (folder=, path=), and gtag.js reports each folder opened there
// as a History API change, real URL and all. A folder shared by link opens by
// that id. Nothing in this file can rewrite those hits; the Redact data
// setting of the GA web stream, for the query parameters folder and path, does.
//
// No SvelteKit imports, so tests/analytics.test.js runs this under plain node.

// Not anchored: a drop link pasted with a full stop, a bracket or a space
// after the id is a 404 page with the id still in its URL.
const UUID = /[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}/i;

/** @param {string} segment */
function decodeSegment(segment) {
	try {
		return decodeURIComponent(segment);
	} catch {
		return segment;
	}
}

/**
 * Whether a path is a drop, share or request page.
 *
 * Decided by the first path segment, so a link with something appended (an
 * error page, but the token is still in it) is covered too: any first segment
 * with a UUID in it, and /s/ or /req/ followed by anything. Segments are
 * decoded first because the router decodes them: /%73/<token> is a share page.
 * @param {string} pathname
 */
export function isLinkPage(pathname) {
	const [first = '', second] = pathname.split('/').filter(Boolean).map(decodeSegment);
	if (UUID.test(first)) return true;
	if (second === undefined) return false;
	const head = first.toLowerCase();
	return head === 's' || head === 'req';
}

/**
 * Whether an in-app navigation enters or leaves a link page. A move within the
 * same page, where only the hash changes, does neither.
 * @param {URL | null | undefined} from
 * @param {URL} to
 */
export function needsFullLoad(from, to) {
	if (
		from &&
		from.origin === to.origin &&
		from.pathname === to.pathname &&
		from.search === to.search
	) {
		return false;
	}
	return Boolean(from && isLinkPage(from.pathname)) || isLinkPage(to.pathname);
}

let gaOnSite = false;
let started = false;
/** Asks before a full page load that would end a running transfer. */
let confirmLeave = () => true;

/**
 * Marks GA as running on this site, on every page including a link page, where
 * gtag.js itself is not loaded. startAnalytics does this.
 */
export function separateLinkPages() {
	gaOnSite = true;
}

/**
 * Whether the root layout turns this in-app navigation into a full page load.
 * The transfer guard asks too, so that a running transfer gets one prompt.
 * @param {URL | null | undefined} from
 * @param {URL} to
 */
export function forcesFullLoad(from, to) {
	return gaOnSite && needsFullLoad(from, to);
}

/**
 * The root layout's beforeNavigate callback: an in-app move onto or off a link
 * page becomes a full page load, once confirmLeave agrees.
 * @param {{ willUnload: boolean, from?: { url: URL } | null, to?: { url: URL } | null, cancel: () => void }} nav
 */
export function leaveLinkPagesInFull(nav) {
	if (nav.willUnload || !nav.to || !forcesFullLoad(nav.from?.url, nav.to.url)) return;
	nav.cancel();
	if (confirmLeave()) location.href = nav.to.url.href;
}

/**
 * A click on a link onto or off a link page loads it in full, before the router
 * sees the click. Runs in the capture phase on window, so it comes before the
 * router's own listener, while a navigation is loading as much as at rest.
 * Clicks the browser handles itself (another tab, a download, a modifier key)
 * are left alone.
 * @param {MouseEvent} event
 */
function onClick(event) {
	if (event.defaultPrevented || event.button !== 0) return;
	if (event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) return;
	const start = /** @type {any} */ (event.composedPath?.()[0] ?? event.target);
	const a = typeof start?.closest === 'function' ? start.closest('a') : null;
	if (!a || a.hasAttribute('download')) return;
	const target = a.getAttribute('target');
	if (target && target !== '_self') return;
	// An SVG <a> has its href as an SVGAnimatedString.
	const href = typeof a.href === 'string' ? a.href : a.href?.baseVal;
	if (!href) return;
	let to;
	try {
		to = new URL(href, location.href);
	} catch {
		return;
	}
	if (to.origin !== location.origin || !forcesFullLoad(new URL(location.href), to)) return;
	event.preventDefault();
	if (confirmLeave()) location.href = to.href;
}

/**
 * Wrap pushState and replaceState so that no link page URL is written into the
 * history of a page where GA runs, whatever made the move (a redirect, a form,
 * goto() while another navigation loads). The page is loaded in full instead
 * and GA is switched off for what remains of this one, which the router may
 * still render the link page into before it unloads.
 * @param {string} gaId
 */
function guardHistory(gaId) {
	for (const name of /** @type {const} */ (['pushState', 'replaceState'])) {
		const original = history[name];
		history[name] = function (state, unused, url) {
			if (url !== undefined && url !== null) {
				const to = new URL(String(url), location.href);
				if (forcesFullLoad(new URL(location.href), to)) {
					/** @type {any} */ (window)[`ga-disable-${gaId}`] = true;
					if (name === 'pushState') location.assign(to.href);
					else location.replace(to.href);
					return;
				}
			}
			return original.apply(history, /** @type {any} */ ([state, unused, url]));
		};
	}
}

/**
 * Start Google Analytics on this page, if this deployment reports to it, and
 * keep link pages apart from it. The root layout calls this once, from onMount.
 *
 * GA is optional: it runs only for a build made for production (prod is false
 * under `vite dev`, so a dev server cannot report even on a matching hostname),
 * with a GA id configured, on the configured host. Self-hosters and dev or
 * staging traffic stay untracked.
 * @param {{ prod: boolean, gaId?: string, gaHostname?: string, confirm?: () => boolean }} options
 *   confirm asks before a full page load that would end a running transfer.
 * @returns {boolean} whether gtag.js was loaded on this page
 */
export function startAnalytics({ prod, gaId, gaHostname, confirm }) {
	if (!prod || !gaId || location.hostname !== (gaHostname || 'silo.cat')) return false;
	if (started) return false;
	started = true;
	separateLinkPages();
	if (confirm) confirmLeave = confirm;
	window.addEventListener('click', onClick, true);
	guardHistory(gaId);

	if (isLinkPage(location.pathname)) return false;
	const gaScript = document.createElement('script');
	gaScript.async = true;
	gaScript.src = `https://www.googletagmanager.com/gtag/js?id=${gaId}`;
	document.head.appendChild(gaScript);
	const w = /** @type {any} */ (window);
	w.dataLayer = w.dataLayer || [];
	// gtag.js does not define this global itself; the standard snippet does, and
	// this is our copy of it. `$lib/track.js` sends events through it.
	w.gtag = function gtag() {
		w.dataLayer.push(arguments);
	};
	w.gtag('js', new Date());
	w.gtag('config', gaId);
	return true;
}
