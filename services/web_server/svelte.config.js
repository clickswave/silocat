import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

// The adapter is selectable so the same app builds two ways:
//   - default: adapter-cloudflare, for the hosted silo.cat on Cloudflare Pages.
//   - ADAPTER=node: adapter-node, a self-contained Node server for self-hosting
//     (orchestration/docker-compose.selfhost.yml).
// Imported dynamically so each build loads only the adapter it uses: the
// Cloudflare build never pulls in adapter-node, and a self-host build never
// pulls in the Cloudflare adapter.
const useNode = process.env.ADAPTER === 'node';
const adapter = useNode
	? (await import('@sveltejs/adapter-node')).default
	: (await import('@sveltejs/adapter-cloudflare')).default;

/**
 * Content Security Policy.
 *
 * Declared here rather than hand-written in `_headers` because SvelteKit emits
 * an inline hydration bootstrap on every page whose contents are generated per
 * request (base path, public env, serialized data). That script cannot be
 * hashed by hand, and a policy written without it blocks hydration on every
 * page in the app: verified in a browser, it did exactly that. Configured here,
 * SvelteKit hashes its own script and merges it into these directives.
 *
 * `mode: 'hash'` rather than 'nonce' because pages are prerendered, and
 * SvelteKit refuses nonce mode with prerendering (a nonce baked into static HTML
 * is not a nonce).
 *
 * Everything else on the page loads from a URL, so `'self'` plus the specific
 * third parties covers it. Notable entries:
 *   'wasm-unsafe-eval'  libsodium. Without it every encrypt and decrypt fails.
 *   blob:               the crypto worker, and object URLs for decrypted previews.
 *   *.r2.cloudflarestorage.com  presigned chunk PUT and GET.
 *   challenges.cloudflare.com   Turnstile, on signin and signup.
 *   *.razorpay.com              checkout.
 *   *.google-analytics.com, *.analytics.google.com, *.googletagmanager.com
 *                               (img-src, connect-src) Where Google Analytics,
 *                               which the layout loads only on silo.cat, sends
 *                               its hits and gtag.js its tag diagnostics.
 *                               Leaving these out blocked GA from the end of
 *                               August to 2026-11-08 without any error a visitor
 *                               would see, and the traffic numbers fell to
 *                               almost nothing. Refusing the diagnostics alone
 *                               achieves nothing: gtag.js sends them again as
 *                               /gtag/js?...&is_td=1, which script-src cannot
 *                               tell apart from the loader. They carry the
 *                               page's path, which is why GA does not run on
 *                               drop, share and request pages at all (see
 *                               src/lib/analytics.js).
 *   www.googletagmanager.com/gtag/js
 *                               (script-src) The one script GA needs, by path.
 *                               The whole host would also run any Tag Manager
 *                               container (/gtm.js?id=...), including one an
 *                               attacker owns, on the pages where people type
 *                               decryption passwords. If the Google tag ever
 *                               gets a second destination, such as Google Ads,
 *                               gtag.js also loads /gtag/destination and that
 *                               path goes here.
 */
/** @type {import('@sveltejs/kit').KitConfig['csp']} */
const csp = {
	mode: 'hash',
	directives: {
		'default-src': ['self'],
		'script-src': [
			'self',
			'wasm-unsafe-eval',
			'https://challenges.cloudflare.com',
			'https://checkout.razorpay.com',
			'https://www.googletagmanager.com/gtag/js'
		],
		'style-src': ['self', 'unsafe-inline'],
		'img-src': [
			'self',
			'data:',
			'blob:',
			'https://*.google-analytics.com',
			'https://*.googletagmanager.com'
		],
		'font-src': ['self'],
		'worker-src': ['self', 'blob:'],
		'connect-src': [
			'self',
			'blob:',
			'https://*.r2.cloudflarestorage.com',
			'https://api.silo.cat',
			'https://challenges.cloudflare.com',
			'https://lumberjack.razorpay.com',
			'https://api.razorpay.com',
			'https://*.google-analytics.com',
			'https://*.analytics.google.com',
			'https://*.googletagmanager.com'
		],
		// 'self' is required by the download sink: the service-worker tier starts a
		// download by pointing a hidden iframe at a same-origin /_stream/<id> URL.
		// Without it the browser refuses to frame that URL and the download never
		// begins, with the stream already written and no error the app can see.
		// Verified in a browser; it is not obvious from reading either file.
		'frame-src': ['self', 'https://challenges.cloudflare.com', 'https://api.razorpay.com'],
		'form-action': ['self'],
		'base-uri': ['none'],
		'object-src': ['none'],
		'frame-ancestors': ['none']
	}
};

/** @type {import('@sveltejs/kit').Config} */
const config = {
	kit: {
		adapter: adapter(),
		csp,
		// Generated-output dir. Overridable so a build can side-step a
		// `.svelte-kit` left root-owned by a previous container build.
		outDir: process.env.SVELTEKIT_OUT_DIR || '.svelte-kit'
	},
	preprocess: [vitePreprocess()]
};

export default config;
