/**
 * The entry name for a file in a zip the browser builds for a download.
 *
 * File and folder names are whatever the uploader sent: an anonymous drop, a
 * share owner, or an outsider uploading through a request link. JSZip stores an
 * entry name exactly as given, so a file called "../../Startup/update.bat", or
 * a folder called "..", made a zip that writes outside the folder the recipient
 * extracts into with any tool that follows "..". The API refuses such names
 * now, but older files keep the names they were stored with.
 *
 * So each folder in `folderPath` and the file name become one safe segment
 * each: separators and control characters turn into "_", and a segment that is
 * only dots and spaces (Windows reads "..." and ".. " as "..") becomes "_".
 * A segment that starts with a drive ("C:" or "C:evil.bat") gets "_" in place
 * of the colon, since a Windows tool that joins it onto the extract folder
 * lands on that drive instead. The fullwidth forms of "/", "\", "." and ":"
 * count as the real ones, because Windows maps them to ASCII when a tool hands
 * the name to an ANSI file API. Ordinary names pass through unchanged.
 *
 * @param {string | null | undefined} folderPath folders from the shared root, joined with "/"
 * @param {string} name the file's name
 * @returns {string}
 */
export function zipEntryPath(folderPath, name) {
	const dirs = folderPath
		? String(folderPath)
				.split('/')
				.filter((part) => part !== '')
				.map(segment)
		: [];
	return [...dirs, segment(name ?? '')].join('/');
}

// Separators, their fullwidth and math look-alikes, and control characters.
// eslint-disable-next-line no-control-regex
const UNSAFE = /[\\/\uff0f\uff3c\u2215\u2216\u0000-\u001f\u007f-\u009f]/g;

/** @param {string} part */
function segment(part) {
	const clean = String(part)
		.replace(UNSAFE, '_')
		.replace(/^([A-Za-z])[:\uff1a]/, '$1_');
	return /^[.\uff0e\s]*$/.test(clean) ? '_' : clean;
}
