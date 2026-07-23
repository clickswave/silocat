<script>
	import { page } from '$app/stores';
	import { SITE, canonicalFor, absoluteUrl } from '$lib/seo.js';

	let {
		title = '',
		description = SITE.defaultDescription,
		canonical = '',
		image = SITE.ogImage,
		type = 'website',
		noindex = false,
		keywords = '',
		schema = null
	} = $props();

	const fullTitle = $derived(title ? title : SITE.defaultTitle);
	const canonicalUrl = $derived(canonical || canonicalFor($page.url.pathname));
	const ogImageUrl = $derived(absoluteUrl(image));
	// schema can be a single object or an array of objects
	const schemas = $derived(schema ? (Array.isArray(schema) ? schema : [schema]) : []);
</script>

<svelte:head>
	<title>{fullTitle}</title>
	<meta name="description" content={description} />
	{#if keywords}<meta name="keywords" content={keywords} />{/if}

	{#if noindex}
		<meta name="robots" content="noindex, nofollow" />
	{:else}
		<meta name="robots" content="index, follow" />
		<link rel="canonical" href={canonicalUrl} />
	{/if}

	<!-- Open Graph -->
	<meta property="og:site_name" content={SITE.name} />
	<meta property="og:type" content={type} />
	<meta property="og:title" content={fullTitle} />
	<meta property="og:description" content={description} />
	<meta property="og:url" content={canonicalUrl} />
	<meta property="og:locale" content={SITE.locale} />
	<meta property="og:image" content={ogImageUrl} />
	<meta property="og:image:width" content={String(SITE.ogImageWidth)} />
	<meta property="og:image:height" content={String(SITE.ogImageHeight)} />
	<meta property="og:image:alt" content={fullTitle} />

	<!-- Twitter -->
	<meta name="twitter:card" content="summary_large_image" />
	<meta name="twitter:title" content={fullTitle} />
	<meta name="twitter:description" content={description} />
	<meta name="twitter:image" content={ogImageUrl} />

	{#each schemas as s}
		{@html `<script type="application/ld+json">${JSON.stringify(s).replace(/</g, '\\u003c')}<\/script>`}
	{/each}
</svelte:head>
