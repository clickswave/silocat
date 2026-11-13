// Conversion tracking for silo.cat.
//
// GA4 is loaded in +layout.svelte, which until now reported pageviews and
// nothing else: no event told us whether a visitor actually dropped a file,
// shared it, or paid. These helpers name the few moments that answer that.
//
// Safe to call anywhere. Every helper no-ops during SSR, on a self-hosted
// deployment, and in any environment where the layout's GA gates did not fire,
// because `window.gtag` only exists once GA is genuinely running.

import { capabilityPageFields } from './analytics.js';

/**
 * Send a GA4 event. On a drop, share or request page the event names the route
 * rather than the link, the same as the page view (see $lib/analytics.js).
 * @param {string} name snake_case event name
 * @param {Record<string, string | number | boolean>} [params]
 */
export function track(name, params = {}) {
	if (typeof window === 'undefined') return;
	try {
		const g = /** @type {any} */ (window).gtag;
		if (typeof g === 'function')
			g('event', name, { ...params, ...capabilityPageFields(location.href) });
	} catch {
		// Analytics must never break the page.
	}
}

/**
 * A signup / CTA click. `location` names the surface that earned it, which is
 * the point: it tells us which moment does the convincing.
 * @param {string} location e.g. 'hero', 'pricing_pro', 'quota_exhausted'
 * @param {Record<string, string | number | boolean>} [extra]
 */
export function trackCta(location, extra = {}) {
	track('cta_click', { location, ...extra });
}

/**
 * The core anonymous-drop funnel. Most of silo.cat's traffic never signs in, so
 * these three are the only view we get of whether the product works for them.
 *
 * `stage` is one of:
 *   'selected'  files chosen, before any encryption or upload
 *   'encrypted' client-side encryption finished
 *   'uploaded'  every chunk landed and a share link exists
 *   'failed'    the drop did not complete
 * @param {'selected' | 'encrypted' | 'uploaded' | 'failed'} stage
 * @param {Record<string, string | number | boolean>} [extra]
 */
export function trackDrop(stage, extra = {}) {
	track('drop_' + stage, extra);
}
