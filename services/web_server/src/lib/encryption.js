// Encryption is chosen per file, so a folder only counts as encrypted when it
// has files and every one of them is. An empty folder, or one with a single
// plaintext file among encrypted ones, is not.

/** @param {Array<{ encrypted?: boolean } | null | undefined> | null | undefined} files */
export function allEncrypted(files) {
	return Array.isArray(files) && files.length > 0 && files.every((f) => f?.encrypted === true);
}
