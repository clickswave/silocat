/**
 * Share-link helpers.
 *
 * The design promotes sharing to a first-class action: copy-link and share
 * buttons sit directly on cards, rows and dashboard recents rather than hiding
 * inside a kebab menu. That means "copy link" has to work in one click even
 * when the item is not shared yet, so this turns sharing on first when needed.
 */
import axios from 'axios';
import { toast } from '$lib/toast.js';

/** Absolute URL for a share token. */
export function shareUrl(token) {
	if (!token) return '';
	const origin = typeof window !== 'undefined' ? window.location.origin : 'https://silo.cat';
	return `${origin}/s/${token}`;
}

/** Current share state for a file or folder, or null when it has never been shared. */
export async function getShareInfo(id) {
	try {
		const { data } = await axios.get(`/api/v1/sanctum/file/share/info/${id}`);
		return data?.success?.data ?? null;
	} catch {
		return null;
	}
}

/**
 * Copy a usable link for `item` to the clipboard, enabling public sharing first
 * if it is currently off. Returns the URL, or null when it could not be shared.
 */
export async function copyShareLink(item, type = 'file') {
	try {
		let info = await getShareInfo(item.id);
		let token = info?.share_token || info?.token;

		if (!token || info?.share_type === 'off' || info?.public_access === false) {
			const { data } = await axios.post('/api/v1/sanctum/file/share/toggle', {
				[type === 'folder' ? 'folder_id' : 'file_id']: item.id,
				share_type: 'public'
			});
			token = data?.success?.data?.share_token || data?.success?.data?.token || token;
		}

		if (!token) {
			toast.error('Could not create a link', 'Try opening share settings for this item.');
			return null;
		}

		const url = shareUrl(token);
		await navigator.clipboard.writeText(url);
		toast.success('Link copied', 'Anyone with it can download the file.');
		return url;
	} catch {
		toast.error('Could not copy the link', 'Check your connection and try again.');
		return null;
	}
}
