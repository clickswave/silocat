/**
 * Resolve the theme before first paint.
 *
 * Deliberately an external file rather than an inline script in app.html.
 * Inline would need either 'unsafe-inline' (which makes the whole script-src
 * decorative) or a SHA-256 pin in the CSP. The pin was tried and is a bad trade:
 * it lives in a different file from the code it hashes, nothing enforces that
 * the two agree, and when they drift the failure is silent. Every visitor just
 * lands on the :root default with no error anyone would notice. As its own file
 * it is covered by `script-src 'self'` and the whole class of problem is gone.
 *
 * Render-blocking on purpose: it must set data-theme before the body paints, or
 * light-theme users get a dark flash. It is a few hundred bytes, same-origin,
 * and cached.
 */
(function () {
	try {
		var stored = localStorage.getItem('theme');
		var theme =
			stored ||
			(window.matchMedia && window.matchMedia('(prefers-color-scheme: light)').matches
				? 'light'
				: 'dark');
		document.documentElement.setAttribute('data-theme', theme);

		var meta = document.createElement('meta');
		meta.name = 'theme-color';
		meta.content = theme === 'light' ? '#fafafa' : '#0b0b0d';
		document.head.appendChild(meta);
	} catch (e) {
		/* private mode or storage disabled: :root already carries the dark default */
	}
})();
