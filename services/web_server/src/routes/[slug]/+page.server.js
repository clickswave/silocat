import { ApiServerClient, ApiServerRoutes } from '$lib/network.js';

function formatSize(bytes) {
	if (!bytes || bytes < 0) return '0 B';
	const k = 1024;
	const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
	const i = Math.min(Math.floor(Math.log(bytes) / Math.log(k)), sizes.length - 1);
	return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
}

// Server-side fetch of the shared resource's metadata so social/link-preview
// crawlers (which do not run JS) get real Open Graph tags describing the file.
// The interactive page still re-fetches client-side in onMount; this load only
// powers the <svelte:head> preview. Best-effort: any failure -> generic preview.
export const load = async ({ params }) => {
	const fallback = {
		og: {
			title: 'Secure download on SiloCat',
			description:
				'A file shared securely and stored on silo.cat, the zero-knowledge encrypted cloud.'
		}
	};

	try {
		const res = await ApiServerClient.post(ApiServerRoutes.fetchResource, {
			id: params.slug
		}).then((r) => r.data);

		const d = res?.data;
		if (!d) return fallback;

		if (d.resource_type === 'file' && d.file) {
			const name = d.file.name || 'File';
			const size = formatSize(Number(d.file.size) || 0);
			return {
				og: {
					title: `${name} — shared on SiloCat`,
					description: `${name} (${size}). Encrypted file stored on silo.cat — download it securely.`
				}
			};
		}

		if (d.resource_type === 'folder' && d.folder) {
			const name = d.folder.name || 'Folder';
			const files = Array.isArray(d.files) ? d.files : [];
			const totalSize = formatSize(files.reduce((sum, f) => sum + (Number(f?.size) || 0), 0));
			const count = files.length;
			const label = count === 1 ? '1 file' : `${count} files`;
			return {
				og: {
					title: `${name} — shared on SiloCat`,
					description: `${label} (${totalSize}) stored on silo.cat — download them securely from this encrypted vault.`
				}
			};
		}

		return fallback;
	} catch (e) {
		console.error('[SHARE_OG_LOAD]', e?.response?.status || e?.message);
		return fallback;
	}
};
