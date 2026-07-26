/**
 * One price table for the whole product.
 *
 * The marketing page and the in-app billing page used to keep separate copies
 * of these numbers, which is how they drift. Both import this now. It mirrors
 * `calculate_price()` in api_switch: annual is ~10 months, so two are free.
 */

export const SYMBOL = { USD: '$', EUR: '€', INR: '₹' };

export const PRICES = {
	USD: { plus: { monthly: 4, annual: 39 }, pro: { monthly: 10, annual: 96 }, free: { monthly: 0, annual: 0 } },
	EUR: { plus: { monthly: 4, annual: 39 }, pro: { monthly: 9, annual: 90 }, free: { monthly: 0, annual: 0 } },
	INR: {
		plus: { monthly: 349, annual: 3490 },
		pro: { monthly: 899, annual: 8990 },
		free: { monthly: 0, annual: 0 }
	}
};

/** Storage each plan grants, for copy that has to agree with the backend. */
export const PLAN_STORAGE = {
	free: '10 GB',
	plus: '200 GB',
	pro: '2 TB'
};

/** `$96`, `₹8,990`: grouped for INR, where the numbers get long. */
export function formatPrice(currency, amount) {
	const symbol = SYMBOL[currency] || '$';
	const grouped = currency === 'INR' ? amount.toLocaleString('en-IN') : String(amount);
	return `${symbol}${grouped}`;
}

/** Currency guess from the account's country; the user can always override. */
const EUROZONE = new Set([
	'austria', 'at', 'belgium', 'be', 'bulgaria', 'bg', 'croatia', 'hr', 'cyprus', 'cy',
	'estonia', 'ee', 'finland', 'fi', 'france', 'fr', 'germany', 'de', 'greece', 'gr',
	'ireland', 'ie', 'italy', 'it', 'latvia', 'lv', 'lithuania', 'lt', 'luxembourg', 'lu',
	'malta', 'mt', 'netherlands', 'nl', 'portugal', 'pt', 'slovakia', 'sk', 'slovenia', 'si',
	'spain', 'es'
]);

export function currencyForCountry(country) {
	const c = (country || '').toLowerCase();
	if (c === 'india' || c === 'in') return 'INR';
	if (EUROZONE.has(c)) return 'EUR';
	return 'USD';
}

/**
 * Gateway amounts are stored in minor units (paise, cents). Render them as
 * money: `formatMinor('INR', 89900)` -> `₹899.00`.
 */
export function formatMinor(currency, minorAmount) {
	const code = (currency || 'USD').toUpperCase();
	const symbol = SYMBOL[code] || '$';
	const major = (Number(minorAmount) || 0) / 100;
	const grouped = major.toLocaleString(code === 'INR' ? 'en-IN' : 'en-US', {
		minimumFractionDigits: 2,
		maximumFractionDigits: 2
	});
	return `${symbol}${grouped}`;
}
