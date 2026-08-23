import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
	plugins: [sveltekit()],
	// Dep-optimizer cache. Overridable so a dev server can side-step a
	// `node_modules/.vite` left root-owned by a previous container build.
	cacheDir: process.env.VITE_CACHE_DIR || 'node_modules/.vite',
	server: {
		watch: {
			// The dev container mounts this whole directory, so anything written
			// here from the host lands inside its watcher. A production build using
			// SVELTEKIT_OUT_DIR (which exists because `.svelte-kit` can be left
			// root-owned by a container build) rewrites tsconfig.json in that dir,
			// and Vite responds with "changed tsconfig file detected", clears its
			// cache and full-reloads, in a loop, until the dev server is unusable.
			// Vite already ignores the default `.svelte-kit`; the alternates need
			// saying explicitly.
			ignored: ['**/.svelte-kit-*/**', '**/build/**']
		}
	}
});
