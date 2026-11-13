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
// /home/files keeps the open folder's id and the names of the folders above it
// in its query (folder=, path=), and gtag.js reports each folder opened there
// as a History API change, real URL and all. A folder shared by link opens by
// that id. Nothing in this file can rewrite those hits; the Redact data
// setting of the GA web stream, for the query parameters folder and path, does.
//
// No SvelteKit imports, so tests/analytics.test.js runs this under plain node.

const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

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
 * error page, but the token is still in it) is covered too. Segments are
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

/**
 * The root layout calls this once it knows GA runs on this site, on every page
 * including a link page, where gtag.js itself is not loaded.
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
