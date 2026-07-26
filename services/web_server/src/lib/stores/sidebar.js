/**
 * Sidebar collapse state. Persisted so the rail stays how the user left it
 * across navigations and sessions (design: `localStorage['silocat-sidebar-collapsed']`,
 * '1' / '0').
 */
import { writable } from 'svelte/store';
import { browser } from '$app/environment';

const LS_KEY = 'silocat-sidebar-collapsed';

function initial() {
	if (!browser) return false;
	try {
		return localStorage.getItem(LS_KEY) === '1';
	} catch {
		return false;
	}
}

export const sidebarCollapsed = writable(initial());

export function toggleSidebar() {
	if (!browser) return;
	sidebarCollapsed.update((v) => {
		const next = !v;
		try {
			localStorage.setItem(LS_KEY, next ? '1' : '0');
		} catch {
			/* private mode: state stays in memory for this session */
		}
		return next;
	});
}
