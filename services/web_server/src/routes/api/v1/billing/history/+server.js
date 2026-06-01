
import { json } from '@sveltejs/kit';
import { ApiServerClient } from '$lib/network.js';

export async function GET({ locals }) {
    let user = await locals.session.user.get();
    if (!user) {
        return json({ error: 'Unauthorized' }, { status: 401 });
    }

    try {
        // We pass the user_id implicitly via the token/session handling in the backend
        // leveraging the existing client structure (headers setup in network.js or similar)
        // Actually, ApiServerClient usually needs the auth header injected if not global.
        // Checking network.js or prior proxy usage (e.g. order/+server.js uses enhancedPayload but that's POST).
        // For GET requests, if the backend relies on X-Api-Key or similar from the session manually?
        // Wait, backend uses `Extension(UserTokenData)` which implies it parses the session token or similar.
        // Usually `ApiServerClient` in SvelteKit backend proxy context might not auto-attach user context unless configured.
        // Let's check `src/lib/network.js` or how other GETs are done.

        let response = await ApiServerClient.get(`/billing/history?user_id=${user.id}`)
            .then(res => res.data);

        // NOTE: Backend extracts user from token normally, but if we are proxying, 
        // we might be passing the cookie or just trusting the internal network.
        // Looking at `api_switch/src/routes/mod.rs`: .layer(from_fn(middlewares::authority_sign_check))
        // And user/register_personal.rs uses `Extension(UserTokenData)`.
        // This usually comes from a middleware that validates a token.
        // In `api/v1/billing/order/+server.js` I saw `enhancedPayload { ...payload, user_id: user.id }`.

        // Let's assume for GET, we might need to pass header or query param if global middleware doesn't pick up cookie. 
        // Actually, `ApiServerClient` is axios instance.
        // If I look at `api/v1/billing/order`, it just POSTs.

        // Correction: Backend expects Authentication? 
        // `routes/mod.rs` has `from_fn(middlewares::authority_sign_check)`.
        // Wait, `authority_sign_check` usually checks signature??
        // Ah, `validate_shadow_user`... 

        // Let's look at `api/v1/shadow/file/+server.js` if it exists to see how GET is handled or `api/v1/billing/order` again.
        // `api/v1/billing/order/+server.js`:
        // `let response = await ApiServerClient.post('/billing/order', enhancedPayload)`
        // It injects `user_id`.

        // Backend `history.rs` uses `Extension(UserTokenData)`.
        // This extension is usually populated by a middleware that checks headers/tokens.
        // If I can't easily see that middleware, I might struggle.
        // BUT, `history.rs` implementation I wrote expects `Extension(UserTokenData)`.

        // Let's assume I need to pass `user_id` as header or query if the middleware uses it?
        // Actually, looking at `api_switch` code for `middlewares`, it likely decodes a token or looks up by header.

        // Safest bet: Pass `user_id` in header `X-User-ID` if middleware supports it, OR
        // If the `ApiServerClient` is pre-configured with a master key, and the middleware produces `User` from `user_id` param?
        // Wait, `Extension(UserTokenData)` usually implies the request was fully authenticated as a user.

        // Let's verify `api_switch/src/middlewares/mod.rs` if possible to be sure, OR
        // Just send `user_id` in headers if that's the established pattern (order endpoint injected it in body).

        // Let's check `api_switch/src/middlewares/mod.rs` before writing this file to be 100% sure.
        // Canceling write to check middleware first.

        // For now, I will write a simple version and if it fails I fix it, 
        // but checking middleware is smarter.

        // Actually, I'll write the file assuming I need to pass specific headers if I knew them.
        // `order` endpoint passed `user_id` in BODY.
        // `history` is GET. Can't pass body.
        // Does `history` backend use `Extension(UserTokenData)`? Yes I wrote it that way.

        // I'll pause this write and check middleware.
        return json({});
    } catch (e) {
        return json({});
    }
}
