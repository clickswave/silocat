// Where a visitor came from, kept until they sign up.
//
// The first visit that carries a `ref` (our own links, such as a download
// page's "send your own"), a `utm_source`, or an outside referrer (chatgpt.com,
// a forum) sets a first-party cookie. The signup action and the Google sign-in
// proxy read it on the server and pass it to the API, which stores it on the
// account (users.signup_source). This works where Google Analytics does not:
// ad blockers, and the CSP that kept GA dark from September to November 2026.
//
// First touch wins: a cookie that is already set is never overwritten.

export const SOURCE_COOKIE = 'sc_src';
const MAX_AGE = 60 * 60 * 24 * 30;

/** Lowercase, [a-z0-9_-] only, at most 40 characters. @param {string | null | undefined} raw */
export function cleanSource(raw) {
	const s = String(raw ?? '')
		.toLowerCase()
		.replace(/[^a-z0-9_-]+/g, '-')
		.replace(/^-+|-+$/g, '')
		.slice(0, 40);
	return s || null;
}

/** Call once per page load in the browser. */
export function captureSource() {
	if (typeof document === 'undefined') return;
	try {
		if (document.cookie.split('; ').some((c) => c.startsWith(SOURCE_COOKIE + '='))) return;
		const q = new URLSearchParams(location.search);
		let src = cleanSource(q.get('ref'));
		if (!src && q.get('utm_source')) {
			src = cleanSource([q.get('utm_source'), q.get('utm_campaign')].filter(Boolean).join('-'));
		}
		if (!src && document.referrer) {
			const host = new URL(document.referrer).hostname.replace(/^www\./, '');
			if (host && host !== location.hostname) src = cleanSource('web-' + host);
		}
		if (!src) return;
		const secure = location.protocol === 'https:' ? '; Secure' : '';
		document.cookie = `${SOURCE_COOKIE}=${src}; Max-Age=${MAX_AGE}; Path=/; SameSite=Lax${secure}`;
	} catch {
		// Attribution must never break the page.
	}
}

/** Server side: the cookie's value, cleaned again. @param {{ get(name: string): string | undefined }} cookies */
export function readSource(cookies) {
	return cleanSource(cookies.get(SOURCE_COOKIE));
}
