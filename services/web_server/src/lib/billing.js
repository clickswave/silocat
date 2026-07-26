/**
 * Statuses that mean money actually changed hands.
 *
 * `paid` and `completed` both occur in the orders table depending on which path
 * settled the order, and `success` comes back from the gateway verify step.
 * Anything else (pending, failed, refunded) has no receipt to show.
 *
 * This is a display guard, not the authority. The server decides: order history
 * is filtered by the `order_is_settled()` SQL function, so an unsettled order
 * should not reach this code at all. It is kept because an invoice reference
 * typed straight into the URL bypasses that filter, and rendering a receipt on
 * a guess is the wrong failure.
 *
 * If a gateway ever reports a new settled status, change
 * `services/api_switch/migrations/0040_order_is_settled.sql` first. That is the
 * definition; this mirrors it.
 */
const SETTLED = new Set(['paid', 'completed', 'success']);

/** True when an order represents a completed payment. */
export function isSettled(status) {
	return SETTLED.has(String(status || '').toLowerCase());
}
