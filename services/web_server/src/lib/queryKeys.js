/**
 * Every TanStack query key in one place.
 *
 * They were string literals scattered across five files, and they had drifted
 * three separate ways:
 *
 *   * the Files page invalidated 'fetchStarredFiles' and 'fetchStarredFolders'.
 *     No query has ever registered those names; the Starred screen uses
 *     'starredFiles'/'starredFolders'. Starring something therefore never
 *     refreshed the Starred page.
 *   * the dashboard registered 'fetchRecentFiles' and 'fetchRootFolders', and
 *     the Files page invalidated neither, so uploads and deletes left the
 *     dashboard stale until a hard reload.
 *   * 'sharedFiles'/'sharedFolders' were invalidated only from inside the
 *     component that owns them, so sharing from anywhere else left them stale.
 *
 * A typo in a literal is silent: it invalidates nothing and looks like it
 * worked. Going through these helpers makes the mistake impossible to write.
 *
 * Note that `files` and `folders` are keyed by folder id. They were not, so
 * every folder shared one cache entry and navigating between folders showed the
 * previous folder's contents until the manual invalidation landed.
 */

export const qk = {
	/** Contents of one folder. `null` is the drive root. */
	files: (folderId = null) => ['files', folderId ?? 'root'],
	folders: (parentId = null) => ['folders', parentId ?? 'root'],

	/** Dashboard. */
	recentFiles: ['recentFiles'],
	rootFolders: ['rootFolders'],

	/** The two ResourceList screens. */
	sharedFiles: ['sharedFiles'],
	sharedFolders: ['sharedFolders'],
	starredFiles: ['starredFiles'],
	starredFolders: ['starredFolders'],

	/** Sidebar meter, dashboard tile, Files header. */
	storage: ['storageStats'],

	trashFiles: ['trashFiles'],
	trashFolders: ['trashFolders']
};

/**
 * Invalidate everything that can show a file or folder.
 *
 * Deliberately broad. These lists are small and the refetches are cheap, while
 * the failure this replaces (a screen quietly disagreeing with the server about
 * what the user owns) is expensive and hard to notice. Callers that know they
 * only touched one folder can still invalidate `qk.files(id)` directly.
 */
export function invalidateResources(queryClient, { folderId = undefined } = {}) {
	// Prefix match: ['files'] covers ['files', <any folder>].
	queryClient.invalidateQueries({ queryKey: ['files'] });
	queryClient.invalidateQueries({ queryKey: ['folders'] });
	queryClient.invalidateQueries({ queryKey: qk.recentFiles });
	queryClient.invalidateQueries({ queryKey: qk.rootFolders });
	queryClient.invalidateQueries({ queryKey: qk.sharedFiles });
	queryClient.invalidateQueries({ queryKey: qk.sharedFolders });
	queryClient.invalidateQueries({ queryKey: qk.starredFiles });
	queryClient.invalidateQueries({ queryKey: qk.starredFolders });
	queryClient.invalidateQueries({ queryKey: qk.trashFiles });
	queryClient.invalidateQueries({ queryKey: qk.trashFolders });
	queryClient.invalidateQueries({ queryKey: qk.storage });
}
