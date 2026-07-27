#!/bin/sh
# Choose the nginx config based on whether origin TLS material is present.
#
# The hardened config (nginx-tls.conf) terminates TLS, requires Cloudflare's
# Authenticated Origin Pulls client certificate, and redirects :80 to :443. It
# only works when BOTH of these are true:
#   - tls/origin.pem + tls/origin.key + tls/cloudflare-origin-pull-ca.pem exist
#   - the Cloudflare zone is set to Full (strict) with origin pulls enabled
#
# If the certificates are absent, serving the TLS config would take the site
# down: with the zone still on Flexible, Cloudflare connects on :80 and the
# redirect to :443 becomes a loop. So we fall back to the plaintext config and
# say so loudly, rather than failing to boot or silently looping.
#
# Plaintext mode is NOT the intended end state. See README.md and finding C-1.
set -e

TLS_DIR=/etc/nginx/tls
if [ -s "$TLS_DIR/origin.pem" ] && [ -s "$TLS_DIR/origin.key" ] && [ -s "$TLS_DIR/cloudflare-origin-pull-ca.pem" ]; then
    echo "[nginx] origin TLS material found: serving HTTPS with Authenticated Origin Pulls."
    cp /etc/nginx/available/nginx-tls.conf /etc/nginx/conf.d/default.conf
else
    echo "[nginx] ============================================================"
    echo "[nginx] WARNING: no origin TLS material in $TLS_DIR."
    echo "[nginx] Falling back to PLAINTEXT :80 (Cloudflare Flexible SSL)."
    echo "[nginx] Traffic between Cloudflare and this origin is UNENCRYPTED,"
    echo "[nginx] and the origin is reachable directly with a forged Host"
    echo "[nginx] header, bypassing Cloudflare WAF and Access. This is"
    echo "[nginx] finding C-1 and is not the intended end state."
    echo "[nginx] Fix: see services/nginx/README.md"
    echo "[nginx] ============================================================"
    cp /etc/nginx/available/nginx-plain.conf /etc/nginx/conf.d/default.conf
fi

exec nginx -g 'daemon off;'
