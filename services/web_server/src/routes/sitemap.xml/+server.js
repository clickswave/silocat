import { SITE } from '$lib/seo.js';

// Public, indexable marketing + policy pages only.
const ROUTES = [
	{ path: '/', priority: '1.0', changefreq: 'weekly' },
	{ path: '/pricing', priority: '0.9', changefreq: 'weekly' },
	{ path: '/about', priority: '0.7', changefreq: 'monthly' },
	{ path: '/api', priority: '0.7', changefreq: 'monthly' },
	{ path: '/privacy', priority: '0.6', changefreq: 'monthly' },
	{ path: '/policies/privacy-policy', priority: '0.4', changefreq: 'yearly' },
	{ path: '/policies/terms-of-service', priority: '0.4', changefreq: 'yearly' },
	{ path: '/policies/acceptable-use', priority: '0.3', changefreq: 'yearly' },
	{ path: '/policies/refund-policy', priority: '0.3', changefreq: 'yearly' },
	{ path: '/policies/dmca-policy', priority: '0.3', changefreq: 'yearly' },
	{ path: '/policies/disclaimer', priority: '0.3', changefreq: 'yearly' }
];

export const prerender = true;

export function GET() {
	const urls = ROUTES.map(
		(r) =>
			`  <url>\n    <loc>${SITE.url}${r.path === '/' ? '' : r.path}</loc>\n    <changefreq>${r.changefreq}</changefreq>\n    <priority>${r.priority}</priority>\n  </url>`
	).join('\n');

	const xml = `<?xml version="1.0" encoding="UTF-8"?>\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">\n${urls}\n</urlset>\n`;

	return new Response(xml, {
		headers: {
			'Content-Type': 'application/xml',
			'Cache-Control': 'public, max-age=3600'
		}
	});
}
