import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
	plugins: [sveltekit()],
	// Dep-optimizer cache. Overridable so a dev server can side-step a
	// `node_modules/.vite` left root-owned by a previous container build.
	cacheDir: process.env.VITE_CACHE_DIR || 'node_modules/.vite'
});
