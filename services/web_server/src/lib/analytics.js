// What Google Analytics is told about the page a visitor is on.
//
// Three kinds of silo.cat URL are credentials in themselves:
//   /<id>         a drop. For an unencrypted anonymous drop the id is all it
//                 takes to download the file.
//   /s/<token>    a share link.
//   /req/<token>  a file request: the token lets anyone upload into the
//                 owner's request.
// gtag.js sends the full URL (page_location), the tab title (page_title) and
// the referrer (page_referrer) with every hit, and a drop page's title is the
// file's name. Sent as they are, each visit would copy a working link and the
// file name into the GA property, readable by anyone with access to it for as
// long as the link lives. On these pages GA gets the route and a fixed title
// instead.
//
// Pure functions, no SvelteKit imports, so tests/analytics.test.js runs them
// under plain node.

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
 * The route and title GA gets for a link page, or null for any other page.
 *
 * Decided by the first path segment, so a link with something appended (an
 * error page, but the token is still in it) is covered too. Segments are
 * decoded first because the router decodes them: /%73/<token> is a share page.
 * @param {string} pathname
 * @returns {{ path: string, title: string } | null}
 */
export function capabilityRoute(pathname) {
	const [first = '', second] = pathname.split('/').filter(Boolean).map(decodeSegment);
	if (UUID.test(first)) return { path: '/:id', title: 'Secure download on SiloCat' };
	if (second === undefined) return null;
	const head = first.toLowerCase();
	if (head === 's') return { path: '/s/:token', title: 'Secure download · Silocat' };
	if (head === 'req') return { path: '/req/:token', title: 'Send a file securely · Silocat' };
	return null;
}

/**
 * page_location and page_title for a hit sent from `href`, or an empty object
 * when the page is not a link page and gtag.js can report it as it is.
 * @param {string} href
 * @returns {{ page_location?: string, page_title?: string }}
 */
export function capabilityPageFields(href) {
	const url = new URL(href);
	const route = capabilityRoute(url.pathname);
	if (!route) return {};
	return { page_location: url.origin + route.path, page_title: route.title };
}

/**
 * A referrer with any link page on this host reduced to its route. A link page
 * that sends the visitor on hands the next page its own URL as the referrer.
 * @param {string} referrer
 * @param {string} hostname  the current page's host
 */
function cleanReferrer(referrer, hostname) {
	if (!referrer) return referrer;
	let url;
	try {
		url = new URL(referrer);
	} catch {
		return '';
	}
	if (url.hostname !== hostname) return referrer;
	const route = capabilityRoute(url.pathname);
	return route ? url.origin + route.path : referrer;
}

/**
 * Parameters for `gtag('config', id, ...)` when a page loads.
 *
 * Empty for an ordinary page reached from an ordinary referrer, so those pages
 * report exactly what gtag.js reports on its own.
 * @param {{ href: string, referrer?: string }} page  location.href and document.referrer
 * @returns {{ page_location?: string, page_title?: string, page_referrer?: string }}
 */
export function analyticsConfig({ href, referrer = '' }) {
	/** @type {{ page_location?: string, page_title?: string, page_referrer?: string }} */
	const params = capabilityPageFields(href);
	const cleaned = cleanReferrer(referrer, new URL(href).hostname);
	if (cleaned !== referrer) params.page_referrer = cleaned;
	return params;
}

/**
 * Whether an in-app navigation has to load the next page in full while GA runs.
 *
 * With enhanced measurement on, gtag.js reports every History API change by
 * itself, with the real new URL and the real URL before it, and nothing of ours
 * runs in between. A fresh page load starts a fresh gtag.js that only hears
 * analyticsConfig. A move within the same page (only the hash changes) stays.
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
	return Boolean((from && capabilityRoute(from.pathname)) || capabilityRoute(to.pathname));
}
