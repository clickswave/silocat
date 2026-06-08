// Central SEO config + structured-data builders. Used by the <Seo> component
// and the sitemap endpoint so canonical/OG/JSON-LD stay consistent everywhere.

export const SITE = {
	url: 'https://silo.cat',
	name: 'SiloCat',
	legalName: 'Clickswave Labs Pvt Ltd',
	twitter: '', // no handle yet
	locale: 'en_US',
	themeColor: '#0a0a0c',
	defaultTitle: 'SiloCat: End-to-end encrypted file sharing & cloud storage',
	titleSuffix: 'SiloCat',
	defaultDescription:
		'SiloCat is zero-knowledge, end-to-end encrypted file sharing and cloud storage. Upload up to 20 GB, share an anonymous link, and keep full control. Built for the privacy-conscious.',
	ogImage: '/og-image.png',
	ogImageWidth: 1200,
	ogImageHeight: 630
};

// Normalize a pathname to a clean canonical URL (strip trailing slash + query).
export function canonicalFor(pathname) {
	let p = (pathname || '/').split('?')[0].split('#')[0];
	if (p.length > 1 && p.endsWith('/')) p = p.slice(0, -1);
	return SITE.url + p;
}

export function absoluteUrl(pathOrUrl) {
	if (!pathOrUrl) return SITE.url + SITE.ogImage;
	if (/^https?:\/\//.test(pathOrUrl)) return pathOrUrl;
	return SITE.url + (pathOrUrl.startsWith('/') ? pathOrUrl : '/' + pathOrUrl);
}

// ---- JSON-LD builders -------------------------------------------------------

export function organizationSchema() {
	return {
		'@context': 'https://schema.org',
		'@type': 'Organization',
		name: SITE.name,
		legalName: SITE.legalName,
		url: SITE.url,
		logo: SITE.url + '/icons/icon-512.png',
		description: SITE.defaultDescription
	};
}

export function websiteSchema() {
	return {
		'@context': 'https://schema.org',
		'@type': 'WebSite',
		name: SITE.name,
		url: SITE.url
	};
}

export function softwareApplicationSchema({ priceFrom = '0' } = {}) {
	return {
		'@context': 'https://schema.org',
		'@type': 'SoftwareApplication',
		name: SITE.name,
		applicationCategory: 'SecurityApplication',
		operatingSystem: 'Web',
		url: SITE.url,
		description: SITE.defaultDescription,
		offers: {
			'@type': 'Offer',
			price: priceFrom,
			priceCurrency: 'USD'
		}
	};
}

export function breadcrumbSchema(items) {
	// items: [{ name, path }]
	return {
		'@context': 'https://schema.org',
		'@type': 'BreadcrumbList',
		itemListElement: items.map((it, i) => ({
			'@type': 'ListItem',
			position: i + 1,
			name: it.name,
			item: canonicalFor(it.path)
		}))
	};
}

export function faqSchema(qa) {
	// qa: [{ q, a }]
	return {
		'@context': 'https://schema.org',
		'@type': 'FAQPage',
		mainEntity: qa.map((x) => ({
			'@type': 'Question',
			name: x.q,
			acceptedAnswer: { '@type': 'Answer', text: x.a }
		}))
	};
}
