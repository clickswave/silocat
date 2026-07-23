import { writable } from 'svelte/store';
import { browser } from '$app/environment';

const THEME_COLORS = { dark: '#0b0b0d', light: '#fafafa' };

function initial() {
	if (!browser) return 'dark';
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
