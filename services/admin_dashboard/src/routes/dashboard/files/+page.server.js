import { ApiServerClient } from '$lib/network';

export const load = async ({ locals }) => {
    try {
        if (!locals.session) return {};
        const res = await ApiServerClient.get('/admin/files');
        return {
            files: res.data.data.files || []
        };
    } catch (err) {
        console.error(err);
        return { files: [] };
    }
};
