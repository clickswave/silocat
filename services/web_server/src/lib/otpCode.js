// What a code box keeps of what was typed or pasted. Email verification codes
// are six digits. Password reset codes are eight letters and digits, and the
// server ignores case, spaces and hyphens in them, so they are kept upper case
// with anything else dropped.
/**
 * @param {unknown} raw
 * @param {number} length
 * @param {boolean} [letters]
 */
export function cleanCode(raw, length, letters = false) {
	const typed = String(raw ?? '');
	const kept = letters ? typed.toUpperCase().replace(/[^0-9A-Z]/g, '') : typed.replace(/\D/g, '');
	return kept.slice(0, length);
}
