
import { json } from '@sveltejs/kit';
import { ApiServerClient } from '$lib/network.js';

export async function POST({ request, locals }) {
    try {
        const user = await locals.session.user.get();
        if (!user) {
            return json({ error: 'Unauthorized' }, { status: 401 });
        }

        const payload = await request.json();

        // Forward to backend
        const res = await ApiServerClient.post('/file/share/toggle', {
            ...payload,
            // User ID is extracted from Extension in backend, but remember `ApiServerClient`
            // doesn't automagically send user context unless the backend middleware reads the cookie/token.
            // For `star` endpoint, we passed `user_id` explicitly in the payload because the middleware was bypassed or modified.
            // My implementation of `share.rs` uses `Extension(user): Extension<UserTokenData>`.
            // This usually requires the request to validation middleware.
            // SvelteKit's `ApiServerClient` has `X-Authority-Sign`.
            // The backend `middlewares::authority_sign_check` validates that.
            // BUT `Extension<UserTokenData>` implies `middlewares::user_token_check` or similar ran.
            // If that middleware requires a header like `X-User-Token` or similar, we must pass it.
            // In `star.rs`, I removed `Extension` and passed `user_id`.
            // In `billing`, I passed `user_id` in query/body.
            // To be safe and consistent with my recent fix, I should update `share.rs` to accept `user_id` in payload?
            // OR I should properly attach the user mechanism.
            // Given the pattern I established in `star.rs` to fix the 500, I should probably stick to explicit `user_id`.
            // BUT I already wrote `share.rs` with `Extension`.
            // I'll check `api_switch/src/middlewares/mod.rs` to see how `UserTokenData` is populated.
            // If I can't check it easily, I might get 500s again.
            //
            // Let's assume I need to pass `user_id` and fix the backend to use it, OR 
            // I pass the user ID in a way the middleware expects. 
            //
            // Since I haven't seen `middlewares` code, and `star` failed with `Extension`, 
            // I should proactively refactor `share.rs` to take `user_id` in payload like `star.rs`.
            //
            // Valid strategy: Update `share.rs` to accept `user_id` in payload and remove `Extension`.
            // This guarantees it works without guessing middleware config.

            // I will write the proxy assuming I will fix backend to take user_id.
            user_id: user.id
        });

        if (res.data.status === 200) {
            return json({
                start: Date.now(),
                success: {
                    status: 200,
                    data: res.data.data
                }
            });
        }

        return json(
            {
                status: res.data.status || 500,
                message: res.data.message || 'Failed',
                errors: [res.data.message],
                data: {}
            },
            { status: res.data.status || 500 }
        );

    } catch (err) {
        console.error('[SHARE_TOGGLE_PROXY]', err);
        return json({ error: 'Internal Error' }, { status: 500 });
    }
}
