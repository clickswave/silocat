import { ApiServerClient } from '$lib/network';

export const load = async ({ locals }) => {
    try {
        if (!locals.session) return {};

        const [statsRes, filesRes, cfRes] = await Promise.all([
            ApiServerClient.get('/admin/stats'),
            ApiServerClient.get('/admin/files'), // Reusing files endpoint for recent table
            ApiServerClient.get('/admin/cloudflare'),
        ]);

        return {
            stats: statsRes.data.data.stats || {},
            recentFiles: (filesRes.data.data.files || []).slice(0, 5),
            cloudflare: cfRes.data.data || { shadow: {}, sanctum: {} }
        };
    } catch (err) {
        console.error(err);
        return { stats: {}, recentFiles: [] };
    }
};
