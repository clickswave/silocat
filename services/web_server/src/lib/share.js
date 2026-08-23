/**
 * Share-link helpers.
 *
 * The design promotes sharing to a first-class action: copy-link and share
 * buttons sit directly on cards, rows and dashboard recents rather than hiding
 * inside a kebab menu. That means "copy link" has to work in one click even
 * when the item is not shared yet, so this turns sharing on first when needed.
 *
 * One click to copy and one click to publish are not the same thing, though, and
 * this does both. So when it had to flip the switch it says so in the past tense
 * and hands back an Undo, rather than reporting a copy and leaving the user to
 * discover later that their file went public. Undo is the affordance that makes
 * the one-click flow safe to keep.
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

/** Turn sharing off again for an item. Used by the Undo action below. */
export async function unshare(item, type = 'file') {
	try {
		await axios.post('/api/v1/sanctum/file/share/toggle', {
			[type === 'folder' ? 'folder_id' : 'file_id']: item.id,
			share_type: 'off'
		});
		toast.success('Sharing turned off', 'The link no longer works.');
		return true;
	} catch {
		toast.error('Could not turn sharing off', 'Open share settings for this item.');
		return false;
	}
}

/**
 * Copy a usable link for `item` to the clipboard, enabling public sharing first
 * if it is currently off. Returns the URL, or null when it could not be shared.
 *
 * `onchange` is invoked whenever the item's share state was actually modified
 * (on publish, and again on undo), so callers can refresh their lists.
 */
export async function copyShareLink(item, type = 'file', { onchange } = {}) {
	try {
		let info = await getShareInfo(item.id);
		let token = info?.share_token || info?.token;
		const wasOff = !token || info?.share_type === 'off' || info?.share_type == null;

		if (wasOff) {
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

		if (wasOff) {
			// State changed. Say that first, offer the way back, and only then
			// mention the clipboard.
			onchange?.();
			toast.success('Sharing turned on, link copied', 'Anyone with this link can download it.', {
				action: {
					label: 'Undo',
					onClick: async () => {
						if (await unshare(item, type)) onchange?.();
					}
				}
			});
		} else {
			toast.success('Link copied', 'Anyone with it can download the file.');
		}

		return url;
	} catch {
		toast.error('Could not copy the link', 'Check your connection and try again.');
		return null;
	}
}
