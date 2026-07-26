/**
 * The client IP, as api_switch should see it.
 *
 * api_switch rate-limits per IP, so it needs the caller's address rather than
 * the web_server container's. The only safe source is `getClientAddress()`:
 * SvelteKit resolves it through the adapter, which knows which hop is
 * trustworthy. On Cloudflare that is `cf-connecting-ip`, set at the edge and
 * stripped from inbound requests; on adapter-node it is `ADDRESS_HEADER` with
 * `XFF_DEPTH`.
 *
 * Reading a forwarding header off the request here would mean trusting whatever
 * the caller typed, which lets anyone pick their own rate-limit bucket.
 */
export function clientIpHeaders(event) {
	try {
		const ip = event.getClientAddress();
		return ip ? { 'CF-Connecting-IP': ip } : {};
	} catch {
		// getClientAddress throws when the adapter cannot determine an address.
		// Send nothing and let api_switch fall back to the peer.
		return {};
	}
}
