// The headers in _headers, for the pages this server renders. Cloudflare Pages
// applies _headers only to files it serves itself, so every server-rendered page
// went out with the CSP that SvelteKit sets and none of these. Keep the two lists
// the same; tests/headers.test.js checks. The CSP stays in svelte.config.js, for
// the reason _headers gives.
//
// Referrer-Policy is strict-origin, not the browsers' default of
// strict-origin-when-cross-origin, so that no page here hands its path and query
// to the next page here either. Some of those are credentials (a drop, share or
// request link) or name someone's folders, and the next page may run Google
// Analytics, which reads document.referrer for itself. Other sites were already
// sent only the origin.
export const SSR_SECURITY_HEADERS = {
	'X-Frame-Options': 'DENY',
	'X-Content-Type-Options': 'nosniff',
	'Referrer-Policy': 'strict-origin',
	'Permissions-Policy': 'geolocation=(), microphone=(), camera=()',
	'Strict-Transport-Security': 'max-age=31536000; includeSubDomains'
};
