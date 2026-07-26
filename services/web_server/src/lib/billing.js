/**
 * Statuses that mean money actually changed hands.
 *
 * `paid` and `completed` both occur in the orders table depending on which path
 * settled the order, and `success` comes back from the gateway verify step.
 * Anything else (pending, failed, refunded) has no receipt to show.
 */
const SETTLED = new Set(['paid', 'completed', 'success']);

/** True when an order represents a completed payment. */
export function isSettled(status) {
	return SETTLED.has(String(status || '').toLowerCase());
}
