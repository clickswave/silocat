export async function load({ fetch }) {
    try {
        const response = await fetch('/api/v1/sanctum/file/list');
        const result = await response.json();

        const files = result?.data?.files || result?.success?.data?.files || [];
        return { files };
    } catch (e) {
        console.error('Dashboard load error:', e);
        return { files: [] };
    }
}
