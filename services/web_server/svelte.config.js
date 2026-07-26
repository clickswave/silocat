import adapter from '@sveltejs/adapter-cloudflare';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

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
