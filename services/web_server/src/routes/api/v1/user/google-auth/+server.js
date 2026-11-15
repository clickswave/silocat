
import { json } from '@sveltejs/kit';
import { ApiServerClient } from '$lib/network.js';
import { readSource } from '$lib/source.js';
import { takeOAuthState } from '$lib/server/oauthState.js';

export async function POST({ request, locals, cookies }) {
    try {
        let payload = await request.json();
        // Only the browser that started this sign-in at /auth/google holds the
        // state Google sent back. A code from anyone else's sign-in is refused
        // before it is exchanged, so it can never become a session here.
        const { state, ...rest } = payload ?? {};
        if (!takeOAuthState(cookies, state)) {
            return json(
                { status: 400, message: 'This sign-in link has expired. Please try again.', errors: [], data: {} },
                { status: 400 }
            );
        }
        payload = rest;
        // Forward the user's real country (Cloudflare sets CF-IPCountry on this
        // request). The API can't geolocate the user since the call is proxied.
        payload.client_country = request.headers.get('cf-ipcountry') || null;
        // First-touch source (see $lib/source.js); only used if this creates the account.
        payload.source = readSource(cookies);
        const response = await ApiServerClient.post('/user/google-auth', payload);

        if (response.status === 200) {
            const user = response.data.data; // token_data returns user in data field? check login.rs response format.
            // login::handle returns: respond(200, "Login successful", vec![], json!(token_data));
            // token_data is the object.
            // So response.data is { status: 200, message: "...", errors: [], data: token_data }

            await locals.session.user.set(user);
            return json(response.data);
        }

        return json(response.data, { status: response.status });

    } catch (err) {
        if (err.response) {
            return json(err.response.data, { status: err.response.status });
        }
        console.error('[POST_GOOGLE_AUTH]', err);
        return json({ error: 'Failed to authenticate with Google' }, { status: 500 });
    }
}
