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

/** @type {import('@sveltejs/kit').Config} */
const config = {
	kit: {
		adapter: adapter(),
		// Generated-output dir. Overridable so a build can side-step a
		// `.svelte-kit` left root-owned by a previous container build.
		outDir: process.env.SVELTEKIT_OUT_DIR || '.svelte-kit'
	},
	preprocess: [vitePreprocess()]
};

export default config;
