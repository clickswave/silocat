export async function load({ fetch }) {
    try {
        const response = await fetch('/api/v1/sanctum/file/list');
        const result = await response.json();

        // The API structure is { success: { data: { files: [...] } } }
        if (result?.success?.data?.files) {
            return {
                files: result.success.data.files
            };
        }

        console.error("API response invalid:", result);
        return { files: [] };
    } catch (e) {
        console.error("Load function error:", e);
        return { files: [] };
    }
}
