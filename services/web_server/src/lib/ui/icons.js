/**
 * Silocat icon set: the inline 24x24 stroke SVGs drawn for the "Ink & Signal"
 * design. No icon font, no external package (a privacy product should not pull
 * icon payloads from a CDN, and @iconify/svelte ships a runtime we don't need).
 *
 * Each entry is `[paths, options]` where `paths` is an array of shape strings
 * ("d" for paths, or a full element for circles/rects) and options carry the
 * per-icon stroke weight the design specifies (1.2 for oversized empty-state
 * glyphs, 1.6-1.7 for UI chrome, 1.9-2.4 for small checks and closes).
 *
 * `fill: true` marks the two brand marks that are solid, not stroked.
 */

const P = (d) => ({ t: 'path', d });
const C = (cx, cy, r) => ({ t: 'circle', cx, cy, r });
const R = (x, y, width, height, rx) => ({ t: 'rect', x, y, width, height, rx });

export const ICONS = {
	// --- navigation / chrome ---------------------------------------------
	home: { s: [P('M4 11l8-6 8 6v8a1 1 0 0 1-1 1h-4v-6h-6v6H5a1 1 0 0 1-1-1z')], w: 1.6 },
	files: { s: [P('M3 7.5A1.5 1.5 0 0 1 4.5 6h4l2 2.5h9A1.5 1.5 0 0 1 21 10v7.5A1.5 1.5 0 0 1 19.5 19h-15A1.5 1.5 0 0 1 3 17.5z')], w: 1.6 },
	share: { s: [C(17, 6, 2), C(7, 12, 2), C(17, 18, 2), P('M8.8 11L15.2 7M8.8 13l6.4 4')], w: 1.6 },
	star: { s: [P('M12 4.5l2.4 5 5.6.7-4 3.9 1 5.4-5-2.7-5 2.7 1-5.4-4-3.9 5.6-.7z')], w: 1.6 },
	'star-fill': { s: [P('M12 4.5l2.4 5 5.6.7-4 3.9 1 5.4-5-2.7-5 2.7 1-5.4-4-3.9 5.6-.7z')], fill: true },
	trash: { s: [P('M5 7h14M9 7V5h6v2M7 7l1 13h8l1-13')], w: 1.6 },
	billing: { s: [P('M3.5 7h17v10h-17zM3.5 11h17')], w: 1.6 },
	settings: { s: [C(12, 12, 2.6), P('M12 4.5v2M12 17.5v2M4.5 12h2M17.5 12h2M7 7l1.4 1.4M15.6 15.6L17 17M17 7l-1.4 1.4M8.4 15.6L7 17')], w: 1.6 },
	support: { s: [P('M4.5 14a7.5 7.5 0 0 1 15 0'), P('M4.5 14v2.5a1.5 1.5 0 0 0 1.5 1.5h2v-6H6a1.5 1.5 0 0 0-1.5 1.5zM19.5 14v2.5a1.5 1.5 0 0 1-1.5 1.5h-2v-6h2A1.5 1.5 0 0 1 19.5 14z')], w: 1.6 },
	menu: { s: [P('M5 7h14M5 12h14M5 17h14')], w: 1.7 },
	logout: { s: [P('M15 8V6a1 1 0 0 0-1-1H6a1 1 0 0 0-1 1v12a1 1 0 0 0 1 1h8a1 1 0 0 0 1-1v-2M11 12h9M17 9l3 3-3 3')], w: 1.6 },
	moon: { s: [P('M18 14.5A7 7 0 0 1 9.5 6a7 7 0 1 0 8.5 8.5z')], w: 1.7 },
	send: { s: [P('M4.5 12L19 5l-3.5 14-4.2-5.2z'), P('M11.3 13.8L19 5')], w: 1.6 },

	// --- chevrons / arrows -------------------------------------------------
	'chevron-right': { s: [P('M10 6l6 6-6 6')], w: 1.7 },
	'chevron-left': { s: [P('M14 6l-6 6 6 6')], w: 1.7 },
	'chevron-down': { s: [P('M6 10l6 6 6-6')], w: 1.7 },
	'chevrons-left': { s: [P('M13 6l-6 6 6 6M18 6l-6 6 6 6')], w: 1.7 },

	// --- actions -----------------------------------------------------------
	upload: { s: [P('M12 16V8m0 0l-3 3m3-3l3 3M5.5 16.5A3.5 3.5 0 0 1 7 10a5 5 0 0 1 9.6-1A3.7 3.7 0 0 1 19 16.5')], w: 1.7 },
	'upload-lg': { s: [P('M12 17V9m0 0l-3.2 3.2M12 9l3.2 3.2M5.5 17.5A3.5 3.5 0 0 1 7 10.8a5.2 5.2 0 0 1 9.9-1A3.8 3.8 0 0 1 19 17.5')], w: 1.2 },
	'upload-alt': { s: [P('M12 17V9m0 0l-3 3m3-3l3 3M5.5 17.5A3.5 3.5 0 0 1 7 10.8a5.2 5.2 0 0 1 9.9-1A3.8 3.8 0 0 1 19 17.5')], w: 1.6 },
	download: { s: [P('M12 5v9m0 0l-3.2-3.2M12 14l3.2-3.2M6 18h12')], w: 1.7 },
	link: { s: [P('M10 14a3.5 3.5 0 0 1 0-5l2-2a3.5 3.5 0 0 1 5 5l-1 1'), P('M14 10a3.5 3.5 0 0 1 0 5l-2 2a3.5 3.5 0 0 1-5-5l1-1')], w: 1.7 },
	copy: { s: [R(9, 9, 10, 10, 2.5), P('M15 9V6.5A1.5 1.5 0 0 0 13.5 5H6.5A1.5 1.5 0 0 0 5 6.5v7A1.5 1.5 0 0 0 6.5 15H9')], w: 1.7 },
	close: { s: [P('M7 7l10 10M17 7L7 17')], w: 1.9 },
	check: { s: [P('M5 12.5l4.5 4.5L19 7.5')], w: 1.9 },
	'check-sm': { s: [P('M6 12.5l4 4 8-9')], w: 1.9 },
	restore: { s: [P('M5 12a7 7 0 1 0 2.2-5.1M5 5v4h4')], w: 1.6 },
	refresh: { s: [P('M19 12a7 7 0 1 1-2.2-5.1M19 5v4h-4')], w: 1.7 },
	search: { s: [C(11, 11, 5.5), P('M15 15l3.5 3.5')], w: 1.7 },
	'search-empty': { s: [C(11, 11, 6.5), P('M15.7 15.7L20 20M8.6 11h4.8')], w: 1.6 },
	'dots-vertical': { s: [C(12, 6, 1.4), C(12, 12, 1.4), C(12, 18, 1.4)], w: 1.6 },
	eye: { s: [P('M3.5 12S7 6.5 12 6.5 20.5 12 20.5 12 17 17.5 12 17.5 3.5 12 3.5 12z'), C(12, 12, 2.4)], w: 1.7 },
	camera: { s: [P('M4.5 8.5h3l1.5-2h6l1.5 2h3v10h-15z'), C(12, 13, 2.8)], w: 1.6 },
	key: { s: [C(15, 9, 3.4), P('M12.6 11.4L7 17l-2.2.4.4-2.2 5.6-5.6')], w: 1.7 },

	// --- state -------------------------------------------------------------
	lock: { s: [R(6, 11, 12, 8, 2), P('M9 11V8.5a3 3 0 0 1 6 0V11')], w: 1.9 },
	'lock-lg': { s: [R(5, 11, 14, 9, 2.5), P('M8.5 11V8.2a3.5 3.5 0 0 1 7 0V11')], w: 1.7 },
	shield: { s: [P('M12 3.5l7 3v5c0 4.5-3 7.5-7 9-4-1.5-7-4.5-7-9v-5z')], w: 1.6 },
	'shield-check': { s: [P('M12 3.5l7 3v5c0 4.5-3 7.5-7 9-4-1.5-7-4.5-7-9v-5z'), P('M9.5 12l1.8 1.8 3.2-3.6')], w: 1.6 },
	alert: { s: [P('M12 8.5v4M12 16h.01'), C(12, 12, 8)], w: 1.7 },
	spinner: { s: [P('M12 4.5a7.5 7.5 0 1 1-5.3 2.2')], w: 1.9, spin: true },
	'checkbox-on': { s: [R(4.5, 4.5, 15, 15, 4), P('M8.5 12.2l2.4 2.3 4.6-5')], w: 1.7 },
	'checkbox-off': { s: [R(4.5, 4.5, 15, 15, 4)], w: 1.7 },

	// --- folders -----------------------------------------------------------
	folder: { s: [P('M4 7.5A1.5 1.5 0 0 1 5.5 6h3l1.6 2h8.4A1.5 1.5 0 0 1 20 9.5v7A1.5 1.5 0 0 1 18.5 18h-13A1.5 1.5 0 0 1 4 16.5z')], w: 1.6 },
	'folder-wide': { s: [P('M3 7.5A1.5 1.5 0 0 1 4.5 6h3.6l1.9 2.4h10.5A1.5 1.5 0 0 1 22 9.9v7.6A1.5 1.5 0 0 1 20.5 19h-16A1.5 1.5 0 0 1 3 17.5z')], w: 1.6 },
	'folder-open': { s: [P('M3 7.5A1.5 1.5 0 0 1 4.5 6h3.6l1.9 2.4h10.5A1.5 1.5 0 0 1 22 9.9v1.1'), P('M2.6 12.4h18.8l-1.7 6.2a1.5 1.5 0 0 1-1.45 1.1H5.75a1.5 1.5 0 0 1-1.45-1.1z')], w: 1.6 },
	'folder-plus': { s: [P('M4 7.5A1.5 1.5 0 0 1 5.5 6h3l1.6 2H18.5A1.5 1.5 0 0 1 20 9.5v7A1.5 1.5 0 0 1 18.5 18h-13A1.5 1.5 0 0 1 4 16.5z'), P('M12 11v4M10 13h4')], w: 1.6 },
	'folder-move': { s: [P('M4 7.5A1.5 1.5 0 0 1 5.5 6h3l1.6 2h8.4A1.5 1.5 0 0 1 20 9.5v7A1.5 1.5 0 0 1 18.5 18h-13A1.5 1.5 0 0 1 4 16.5z'), P('M10 13h5m0 0l-2-2m2 2l-2 2')], w: 1.6 },
	grid: { s: [R(4.5, 4.5, 6, 6, 1.5), R(13.5, 4.5, 6, 6, 1.5), R(4.5, 13.5, 6, 6, 1.5), R(13.5, 13.5, 6, 6, 1.5)], w: 1.6 },
	list: { s: [P('M5 7h14M5 12h14M5 17h14')], w: 1.6 },

	// --- file type glyphs (deliberately neutral, never per-type colour) ----
	file: { s: [P('M13.5 4.5H7A1.5 1.5 0 0 0 5.5 6v12A1.5 1.5 0 0 0 7 19.5h10a1.5 1.5 0 0 0 1.5-1.5V9.5z'), P('M13.5 4.5v5h5')], w: 1.6 },
	doc: { s: [P('M13.5 4.5H7A1.5 1.5 0 0 0 5.5 6v12A1.5 1.5 0 0 0 7 19.5h10a1.5 1.5 0 0 0 1.5-1.5V9.5z'), P('M13.5 4.5v5h5'), P('M9 13h6M9 16h4')], w: 1.6 },
	image: { s: [P('M4.5 6.5h15v11h-15z'), P('M4.5 14.5l4-4 3.5 3.5 3-2.5 4.5 4')], w: 1.6 },
	video: { s: [P('M4.5 7.5h11v9h-11z'), P('M15.5 11l4-2.5v7L15.5 13z')], w: 1.6 },
	audio: { s: [P('M9 16.5V7l8-1.5v9'), P('M9 16.5a2 2 0 1 1-2.6-1.9'), P('M17 14.5a2 2 0 1 1-2.6-1.9')], w: 1.6 },

	// --- brand marks (solid, not stroked) ----------------------------------
	github: { s: [P('M12 3.5a8.5 8.5 0 0 0-2.7 16.6c.4.1.6-.2.6-.5v-1.8c-2.3.5-2.8-1.1-2.8-1.1-.4-1-1-1.2-1-1.2-.7-.5.1-.5.1-.5.8.1 1.3.9 1.3.9.7 1.3 1.9.9 2.4.7.1-.6.3-1 .6-1.2-1.9-.2-3.8-.9-3.8-4.1 0-.9.3-1.6.8-2.2-.1-.2-.4-1 .1-2.1 0 0 .7-.2 2.3.8a7.9 7.9 0 0 1 4.2 0c1.6-1 2.3-.8 2.3-.8.5 1.1.2 1.9.1 2.1.5.6.8 1.3.8 2.2 0 3.2-1.9 3.9-3.8 4.1.3.3.6.8.6 1.6v2.3c0 .3.2.6.6.5A8.5 8.5 0 0 0 12 3.5z')], fill: true },
	x: { s: [P('M17.5 4h2.6l-5.7 6.5L21 20h-5.3l-4.1-5.4L6.8 20H4.2l6.1-7L3.5 4h5.4l3.8 5 4.8-5z')], fill: true },
	google: {
		raw: '<path fill="#4285F4" d="M21.6 12.2c0-.6-.05-1.2-.16-1.8H12v3.4h5.4a4.6 4.6 0 0 1-2 3v2.5h3.2c1.9-1.7 3-4.3 3-7.1z"/><path fill="#34A853" d="M12 22c2.7 0 5-.9 6.6-2.4l-3.2-2.5c-.9.6-2 1-3.4 1-2.6 0-4.8-1.8-5.6-4.1H3.1v2.6A10 10 0 0 0 12 22z"/><path fill="#FBBC05" d="M6.4 14c-.2-.6-.3-1.3-.3-2s.1-1.4.3-2V7.4H3.1a10 10 0 0 0 0 9.2z"/><path fill="#EA4335" d="M12 5.9c1.5 0 2.8.5 3.8 1.5l2.8-2.8C16.9 3 14.7 2 12 2a10 10 0 0 0-8.9 5.4L6.4 10c.8-2.3 3-4.1 5.6-4.1z"/>'
	},

	// --- drawn in the same hand to cover the rest of the product -----------
	sun: { s: [C(12, 12, 4), P('M12 3.5v2M12 18.5v2M3.5 12h2M18.5 12h2M6.3 6.3l1.4 1.4M16.3 16.3l1.4 1.4M17.7 6.3l-1.4 1.4M7.7 16.3l-1.4 1.4')], w: 1.7 },
	mail: { s: [R(3.5, 5.5, 17, 13, 2), P('M4 7l8 5.5L20 7')], w: 1.6 },
	'mail-check': { s: [P('M20.5 11V7.5a2 2 0 0 0-2-2h-13a2 2 0 0 0-2 2v9a2 2 0 0 0 2 2H13'), P('M4 7l8 5.5L20 7'), P('M15.5 17.5l2 2 4-4.5')], w: 1.6 },
	user: { s: [C(12, 8.5, 3.5), P('M5 19.5a7 7 0 0 1 14 0')], w: 1.6 },
	'user-settings': { s: [C(11, 8.5, 3.3), P('M4.5 19.5a6.6 6.6 0 0 1 9.5-5.9'), C(17.5, 17.5, 2.2), P('M17.5 13.8v1.2M17.5 20v1.2M13.8 17.5H15M20 17.5h1.2')], w: 1.6 },
	plus: { s: [P('M12 5.5v13M5.5 12h13')], w: 1.7 },
	edit: { s: [P('M4.5 19.5h3l9.3-9.3a2.1 2.1 0 0 0-3-3L4.5 16.5z'), P('M14 6.5l3.5 3.5')], w: 1.6 },
	flag: { s: [P('M6 20.5V4.5h10l-1.6 3.2L16 11H6')], w: 1.6 },
	ticket: { s: [P('M4 8.5A1.5 1.5 0 0 1 5.5 7h13A1.5 1.5 0 0 1 20 8.5v2a2 2 0 0 0 0 3.8v2a1.5 1.5 0 0 1-1.5 1.5h-13A1.5 1.5 0 0 1 4 16.3v-2a2 2 0 0 0 0-3.8z'), P('M12 8.5v7')], w: 1.6 },
	tag: { s: [P('M11.4 4.5H4.5v6.9l8.1 8.1a1.5 1.5 0 0 0 2.1 0l4.8-4.8a1.5 1.5 0 0 0 0-2.1z'), C(8, 8, 1.2)], w: 1.6 },
	clock: { s: [C(12, 12, 7.8), P('M12 7.8V12l2.8 1.8')], w: 1.6 },
	bug: { s: [R(7.5, 8.5, 9, 10, 4.5), P('M9.5 8.5a2.5 2.5 0 0 1 5 0M4.5 12h3M16.5 12h3M5.5 8l2 1.4M18.5 8l-2 1.4M5.5 17l2-1.4M18.5 17l-2-1.4')], w: 1.6 },
	chat: { s: [P('M4.5 6.5A1.5 1.5 0 0 1 6 5h12a1.5 1.5 0 0 1 1.5 1.5v8A1.5 1.5 0 0 1 18 16h-6.5L7 19.5V16H6a1.5 1.5 0 0 1-1.5-1.5z')], w: 1.6 },
	lightbulb: { s: [P('M9.2 16.5a5.5 5.5 0 1 1 5.6 0v1.8a1.2 1.2 0 0 1-1.2 1.2h-3.2a1.2 1.2 0 0 1-1.2-1.2z'), P('M9.8 16.5h4.4')], w: 1.6 },
	question: { s: [C(12, 12, 8), P('M9.8 9.8a2.3 2.3 0 1 1 2.9 2.2c-.5.2-.7.6-.7 1.1v.4'), P('M12 16.4h.01')], w: 1.6 },
	crown: { s: [P('M4.5 8.5l2.6 2.4L12 6l4.9 4.9 2.6-2.4V17a1.5 1.5 0 0 1-1.5 1.5H6a1.5 1.5 0 0 1-1.5-1.5z')], w: 1.6 },
	drive: { s: [R(3.5, 4.5, 17, 6, 1.8), R(3.5, 13.5, 17, 6, 1.8), P('M7 7.5h.01M7 16.5h.01')], w: 1.6 },
	inbox: { s: [P('M3.5 13h4l1.2 2.5h6.6L16.5 13h4'), P('M3.5 13l2.4-7A1.5 1.5 0 0 1 7.3 5h9.4a1.5 1.5 0 0 1 1.4 1l2.4 7v4a1.5 1.5 0 0 1-1.5 1.5h-13A1.5 1.5 0 0 1 3.5 17z')], w: 1.6 },
	incognito: { s: [P('M5 11.5l1.8-5A1.5 1.5 0 0 1 8.2 5.5h7.6a1.5 1.5 0 0 1 1.4 1l1.8 5M3.5 11.5h17'), C(8, 15.5, 2.6), C(16, 15.5, 2.6), P('M10.6 15.5h2.8')], w: 1.6 },
	globe: { s: [C(12, 12, 8), P('M4 12h16M12 4a13 13 0 0 1 0 16a13 13 0 0 1 0-16z')], w: 1.6 },
	text: { s: [P('M5 6.5h14M5 12h10M5 17.5h7')], w: 1.7 },
	'shield-star': { s: [P('M12 3.5l7 3v5c0 4.5-3 7.5-7 9-4-1.5-7-4.5-7-9v-5z'), P('M12 8.6l1.2 2.4 2.6.4-1.9 1.8.5 2.6-2.4-1.3-2.4 1.3.5-2.6L8.2 11.4l2.6-.4z')], w: 1.6 },
	'user-shield': { s: [C(11, 8.5, 3.3), P('M4.5 19.5a6.6 6.6 0 0 1 8.3-6.4'), P('M17.5 12.6l3.5 1.4v2.2c0 2.1-1.5 3.5-3.5 4.2-2-.7-3.5-2.1-3.5-4.2V14z')], w: 1.6 },
	laptop: { s: [R(5, 6, 14, 9, 1.8), P('M3 18h18')], w: 1.6 },
	'chart-box': { s: [R(4.5, 4.5, 15, 15, 3), P('M9 15.5v-3M12 15.5v-6M15 15.5v-4')], w: 1.6 },
	'file-list': { s: [P('M13.5 4.5H7A1.5 1.5 0 0 0 5.5 6v12A1.5 1.5 0 0 0 7 19.5h10a1.5 1.5 0 0 0 1.5-1.5V9.5z'), P('M13.5 4.5v5h5'), P('M9 12.5h6M9 15.5h6M9 18h3')], w: 1.6 },
	'file-zip': { s: [P('M13.5 4.5H7A1.5 1.5 0 0 0 5.5 6v12A1.5 1.5 0 0 0 7 19.5h10a1.5 1.5 0 0 0 1.5-1.5V9.5z'), P('M13.5 4.5v5h5'), P('M10 5.5v1.5M11.5 7v1.5M10 8.5V10M11.5 10v1.5M10 11.5V13h1.5v1.5H10')], w: 1.5 },
	'unlock': { s: [R(6, 11, 12, 8, 2), P('M9 11V8.5a3 3 0 0 1 5.7-1.3')], w: 1.7 },
	'select-multiple': { s: [R(8.5, 8.5, 11, 11, 2.5), P('M15.5 5.5H6A1.5 1.5 0 0 0 4.5 7v9.5'), P('M11.5 14l1.8 1.8 3.2-3.6')], w: 1.6 },
	'arrow-left': { s: [P('M19 12H5m0 0l5.5-5.5M5 12l5.5 5.5')], w: 1.7 },
	'arrow-right': { s: [P('M5 12h14m0 0l-5.5-5.5M19 12l-5.5 5.5')], w: 1.7 },
	'arrow-up': { s: [P('M12 19V5m0 0L6.5 10.5M12 5l5.5 5.5')], w: 1.7 },
	'chevron-up': { s: [P('M6 14l6-6 6 6')], w: 1.7 },
	'sort-asc': { s: [P('M12 8l4 4h-8z')], w: 1.6, fill: false },
	dashboard: { s: [R(4.5, 4.5, 6, 6, 1.5), R(13.5, 4.5, 6, 6, 1.5), R(4.5, 13.5, 6, 6, 1.5), R(13.5, 13.5, 6, 6, 1.5)], w: 1.6 },
	warning: { s: [P('M12 4.8L3.8 19h16.4z'), P('M12 10v4M12 16.6h.01')], w: 1.6 },
	'close-circle': { s: [C(12, 12, 8), P('M9.2 9.2l5.6 5.6M14.8 9.2l-5.6 5.6')], w: 1.7 },
	'check-circle': { s: [C(12, 12, 8), P('M8.4 12.2l2.6 2.5 4.6-5.2')], w: 1.7 },
	send: { s: [P('M4.5 12L19 5l-3.5 14-4.2-5.2z'), P('M11.3 13.8L19 5')], w: 1.6 }
};

/**
 * Legacy `ri:*` / `svg-spinners:*` names mapped onto the design set, so the
 * whole app can drop `@iconify/svelte` (a runtime that fetches icon data over
 * the network) in one move while pages are ported one at a time.
 */
export const ICON_ALIASES = {
	'ri:add-line': 'plus',
	'ri:alarm-warning-line': 'warning',
	'ri:arrow-down-s-line': 'chevron-down',
	'ri:arrow-go-back-line': 'restore',
	'ri:arrow-left-line': 'arrow-left',
	'ri:arrow-right-line': 'arrow-right',
	'ri:arrow-right-s-line': 'chevron-right',
	'ri:arrow-up-s-line': 'chevron-up',
	'ri:bank-card-line': 'billing',
	'ri:bar-chart-box-line': 'chart-box',
	'ri:bug-line': 'bug',
	'ri:camera-line': 'camera',
	'ri:chat-3-line': 'chat',
	'ri:check-line': 'check',
	'ri:checkbox-blank-line': 'checkbox-off',
	'ri:checkbox-circle-fill': 'check-circle',
	'ri:checkbox-fill': 'checkbox-on',
	'ri:checkbox-multiple-line': 'select-multiple',
	'ri:close-circle-fill': 'close-circle',
	'ri:close-line': 'close',
	'ri:customer-service-2-line': 'support',
	'ri:dashboard-line': 'dashboard',
	'ri:delete-bin-5-fill': 'trash',
	'ri:delete-bin-7-line': 'trash',
	'ri:delete-bin-line': 'trash',
	'ri:download-2-fill': 'download',
	'ri:download-line': 'download',
	'ri:earth-line': 'globe',
	'ri:edit-line': 'edit',
	'ri:error-warning-fill': 'alert',
	'ri:error-warning-line': 'alert',
	'ri:eye-line': 'eye',
	'ri:eye-off-line': 'eye',
	'ri:file-3-line': 'file',
	'ri:file-copy-line': 'copy',
	'ri:file-fill': 'file',
	'ri:file-line': 'file',
	'ri:file-list-3-line': 'file-list',
	'ri:file-text-fill': 'doc',
	'ri:file-text-line': 'doc',
	'ri:file-zip-line': 'file-zip',
	'ri:film-fill': 'video',
	'ri:film-line': 'video',
	'ri:flag-line': 'flag',
	'ri:folder-3-fill': 'folder',
	'ri:folder-3-line': 'folder',
	'ri:folder-5-fill': 'folder-wide',
	'ri:folder-add-line': 'folder-plus',
	'ri:folder-line': 'files',
	'ri:folder-open-line': 'folder-open',
	'ri:folder-transfer-line': 'folder-move',
	'ri:github-fill': 'github',
	'ri:hard-drive-2-line': 'drive',
	'ri:image-2-fill': 'image',
	'ri:image-2-line': 'image',
	'ri:image-fill': 'image',
	'ri:image-line': 'image',
	'ri:inbox-2-line': 'inbox',
	'ri:key-2-line': 'key',
	'ri:layout-grid-line': 'grid',
	'ri:lightbulb-line': 'lightbulb',
	'ri:links-line': 'link',
	'ri:list-unordered': 'list',
	'ri:loader-4-line': 'spinner',
	'ri:lock-2-line': 'lock-lg',
	'ri:lock-fill': 'lock',
	'ri:lock-unlock-line': 'unlock',
	'ri:logout-box-r-line': 'logout',
	'ri:macbook-line': 'laptop',
	'ri:mail-check-line': 'mail-check',
	'ri:mail-line': 'mail',
	'ri:menu-line': 'menu',
	'ri:moon-line': 'moon',
	'ri:more-2-fill': 'dots-vertical',
	'ri:music-2-line': 'audio',
	'ri:music-fill': 'audio',
	'ri:price-tag-3-line': 'tag',
	'ri:question-line': 'question',
	'ri:refresh-line': 'refresh',
	'ri:search-eye-line': 'search-empty',
	'ri:search-line': 'search',
	'ri:send-plane-2-line': 'send',
	'ri:settings-3-line': 'settings',
	'ri:share-forward-line': 'share',
	'ri:share-line': 'share',
	'ri:shield-check-line': 'shield-check',
	'ri:shield-star-line': 'shield-star',
	'ri:shield-user-line': 'user-shield',
	'ri:spy-line': 'incognito',
	'ri:star-fill': 'star-fill',
	'ri:star-line': 'star',
	'ri:sun-line': 'sun',
	'ri:text': 'text',
	'ri:ticket-line': 'ticket',
	'ri:time-line': 'clock',
	'ri:twitter-x-line': 'x',
	'ri:upload-cloud-2-fill': 'upload-alt',
	'ri:upload-cloud-2-line': 'upload',
	'ri:user-line': 'user',
	'ri:user-settings-line': 'user-settings',
	'ri:user-smile-line': 'user',
	'ri:vip-crown-2-line': 'crown',
	'logos:google-icon': 'google',
	'svg-spinners:ring-resize': 'spinner',
	'svg-spinners:12-dots-scale-rotate': 'spinner'
};

/** Resolve either a design name or a legacy `ri:*` alias to a design name. */
export function resolveIcon(name) {
	if (!name) return 'file';
	return ICON_ALIASES[name] || name;
}

/** File-type glyph for a mime string. Never colour these per type. */
export function glyphForMime(mime = '', name = '') {
	const m = (mime || '').toLowerCase();
	if (m.startsWith('image/')) return 'image';
	if (m.startsWith('video/')) return 'video';
	if (m.startsWith('audio/')) return 'audio';
	if (m.includes('pdf') || m.includes('document') || m.startsWith('text/')) return 'doc';
	const ext = (name || '').split('.').pop()?.toLowerCase();
	if (['png', 'jpg', 'jpeg', 'gif', 'webp', 'avif', 'svg'].includes(ext)) return 'image';
	if (['mp4', 'mov', 'mkv', 'webm', 'avi'].includes(ext)) return 'video';
	if (['mp3', 'wav', 'flac', 'm4a', 'ogg'].includes(ext)) return 'audio';
	if (['pdf', 'doc', 'docx', 'txt', 'md', 'rtf'].includes(ext)) return 'doc';
	return 'file';
}
