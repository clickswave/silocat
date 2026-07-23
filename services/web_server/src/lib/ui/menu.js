import { writable } from 'svelte/store';

/**
 * Singleton floating-menu state. One <Menu /> instance lives in the root
 * layout; any call site (right-click, kebab button) opens it with:
 *
 *   openMenu(event, [{ label, icon, danger, disabled, divider, action }])
 *
 * Coordinates come from the triggering event (click or contextmenu).
 */
export const menuState = writable({ open: false, x: 0, y: 0, items: [] });

export function openMenu(event, items) {
	event.preventDefault();
	event.stopPropagation();
	menuState.set({ open: true, x: event.clientX, y: event.clientY, items });
}

/** Anchor to an element (kebab buttons): opens under its bottom-left corner. */
export function openMenuAt(el, items) {
	const r = el.getBoundingClientRect();
	menuState.set({ open: true, x: r.left, y: r.bottom + 4, items });
}

export function closeMenu() {
	menuState.update((s) => ({ ...s, open: false }));
}
