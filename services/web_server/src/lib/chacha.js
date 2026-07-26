import sodium from 'libsodium-wrappers-sumo';

/**
 * Encrypts a chunk of data using XChaCha20-Poly1305 AEAD.
 * @param chunkBuffer
 * @param key
 * @param nonce
 * @returns {Promise<*>}
 */
export async function encryptChunk(chunkBuffer, key, nonce) {
	await sodium.ready;

	if (!(key instanceof Uint8Array) || key.length !== sodium.crypto_aead_xchacha20poly1305_ietf_KEYBYTES) {
		throw new Error("Invalid encryption key");
	}
	if (!(nonce instanceof Uint8Array) || nonce.length !== sodium.crypto_aead_xchacha20poly1305_ietf_NPUBBYTES) {
		throw new Error("Invalid nonce");
	}

	const ciphertext = sodium.crypto_aead_xchacha20poly1305_ietf_encrypt(
		chunkBuffer,
		null, // optional associated data
		null, // secret nonce increment not used
		nonce,
		key
	);

	return ciphertext;
}

/**
 * Decrypts a chunk of data using XChaCha20-Poly1305 AEAD.
 * @param encryptedBuffer
 * @param key
 * @param nonce
 * @returns {Promise<*>}
 */
export async function decryptChunk(encryptedBuffer, key, nonce) {
	await sodium.ready;

	if (!(key instanceof Uint8Array) || key.length !== sodium.crypto_aead_xchacha20poly1305_ietf_KEYBYTES) {
		throw new Error("Invalid decryption key");
	}
	if (!(nonce instanceof Uint8Array) || nonce.length !== sodium.crypto_aead_xchacha20poly1305_ietf_NPUBBYTES) {
		throw new Error("Invalid nonce");
	}

	try {
		const decrypted = sodium.crypto_aead_xchacha20poly1305_ietf_decrypt(
			null,               // no associated data
			encryptedBuffer,
			null,               // no secret nonce increment
			nonce,
			key
		);

		return decrypted;
	} catch (e) {
		throw new Error("Decryption failed: Invalid key or corrupted data");
	}
}

export async function deriveKeyFromPassword(password, salt) {
	await sodium.ready;

	if (!(salt instanceof Uint8Array) || salt.length !== 16) {
		throw new Error("Invalid salt: must be 16 bytes");
	}

	const keyLength = sodium.crypto_aead_xchacha20poly1305_ietf_KEYBYTES;

	const key = sodium.crypto_pwhash(
		keyLength,                // length of output key (32 bytes)
		password,                 // user's password (string or Uint8Array)
		salt,                     // 16-byte salt
		sodium.crypto_pwhash_OPSLIMIT_MODERATE,
		sodium.crypto_pwhash_MEMLIMIT_MODERATE,
		sodium.crypto_pwhash_ALG_DEFAULT
	);

	return key; // Uint8Array of derived key
}


export function generateSalt() {
	return sodium.randombytes_buf(16); // 128-bit random salt
}

export function generateKey() {
	return sodium.randombytes_buf(sodium.crypto_aead_xchacha20poly1305_ietf_KEYBYTES);
}

export function generateNonce() {
	return sodium.randombytes_buf(sodium.crypto_aead_xchacha20poly1305_ietf_NPUBBYTES);
}
