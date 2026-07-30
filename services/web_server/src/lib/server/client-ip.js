/**
 * The client IP, as api_switch should see it.
 *
 * api_switch rate-limits and IP-bans on this, so it needs the visitor's address
 * rather than the web_server's.
 *
 * The source is `getClientAddress()`: SvelteKit resolves it through the adapter,
 * which knows which hop is trustworthy. On Cloudflare that is `cf-connecting-ip`
 * (set at the edge, stripped from inbound requests); on adapter-node it is
 * `ADDRESS_HEADER` with `XFF_DEPTH`. Reading a forwarding header off the request
 * directly would mean trusting whatever the caller typed, which lets anyone pick
 * their own rate-limit bucket.
 *
 * It is forwarded as X-Client-IP and NOT as CF-Connecting-IP, which is the bug
 * this replaces. api.silo.cat is proxied, so a call from the web_server to
 * api_switch crosses Cloudflare a SECOND time, and Cloudflare sets
 * CF-Connecting-IP itself on that hop -- to the Pages function's egress address.
 * Every value forwarded under that name was therefore overwritten with a
 * Cloudflare IP before it ever reached api_switch, which is why the admin panel
 * listed 2a06:98c0::/29 addresses for every anonymous session, and why per-IP
 * rate limits were effectively one shared bucket.
 *
 * X-Client-IP is safe to trust because api_switch gates every route on a caller
 * secret (authority_sign_check), so only our own backends can set it.
 */
export function clientIpHeaders(event) {
	try {
		const ip = event.getClientAddress();
		return ip ? { 'X-Client-IP': ip } : {};
	} catch {
		// getClientAddress throws when the adapter cannot determine an address.
		// Send nothing and let api_switch fall back to the peer.
		return {};
	}
}
