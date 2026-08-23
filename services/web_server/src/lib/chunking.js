/**
 * One chunk size for every upload path.
 *
 * It was 100 MB, duplicated across four files. That is a poor size for a browser
 * doing client-side crypto:
 *
 *   * a chunk is fully resident while it is sliced, encrypted and PUT, and the
 *     ciphertext is a second copy, so 100 MB chunks meant ~200 MB of churn per
 *     step and made the crypto worker's messages enormous;
 *   * progress only moves once per chunk, so a slow connection sat at the same
 *     percentage for minutes;
 *   * a failed chunk costs the whole chunk again on retry.
 *
 * 16 MB keeps R2 multipart happy (its 5 MB minimum, well under its 5 GB maximum),
 * gives roughly six progress ticks per 100 MB, and caps worst-case resident bytes
 * per in-flight chunk at something a phone can survive.
 *
 * Changing this is safe for new uploads and irrelevant to old ones: chunk
 * boundaries are recorded per file at upload time and downloads follow whatever
 * the server hands back.
 */
export const CHUNK_SIZE = 16 * 1024 * 1024;
