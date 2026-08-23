import { writable } from 'svelte/store';
import { browser } from '$app/environment';

const THEME_COLORS = { dark: '#0b0b0d', light: '#fafafa' };

function initial() {
	if (!browser) return 'dark';
	// app.html has already resolved this before first paint, including the
	// prefers-color-scheme fallback, and written it to the root element. Reading
	// it back is how the store and the DOM stay in agreement; re-deriving it here
	// meant a visitor with no stored preference on a light OS got a light page
	// and a store that said 'dark'.
	const applied = document.documentElement.getAttribute('data-theme');
	if (applied === 'light' || applied === 'dark') return applied;
	return localStorage.getItem('theme') || 'dark';
}

export const theme = writable(initial());

export function setTheme(next) {
	if (!browser) return;
	document.documentElement.setAttribute('data-theme', next);
	localStorage.setItem('theme', next);
	const meta = document.querySelector('meta[name="theme-color"]');
	if (meta) meta.setAttribute('content', THEME_COLORS[next] || THEME_COLORS.dark);
	theme.set(next);
}

export function toggleTheme() {
	if (!browser) return;
	const current = document.documentElement.getAttribute('data-theme') || 'dark';
	setTheme(current === 'dark' ? 'light' : 'dark');
}
