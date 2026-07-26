/**
 * The crypto Silocat actually uses.
 *
 * These parameters are not tunable. They must match the web app byte for byte,
 * because a file encrypted with different parameters is simply undecryptable by
 * anyone else, including its owner. If you are reading this to "optimise" the
 * KDF cost: don't. Changing it silently produces files the website cannot open.
 */
import _sodium from 'libsodium-wrappers-sumo';

let sodium;

/** Resolves once libsodium's WASM is ready. Every export awaits this. */
export async function ready() {
	if (!sodium) {
		await _sodium.ready;
		sodium = _sodium;
	}
	return sodium;
}

/**
 * Derive a 32-byte file key from a password.
 *
 * Argon2id at libsodium's MODERATE limits. Deliberately expensive: this is the
 * only thing standing between a leaked ciphertext blob and its plaintext.
 */
export async function deriveKey(password, salt) {
	const s = await ready();
	return s.crypto_pwhash(
		32,
		password,
		new Uint8Array(salt),
		s.crypto_pwhash_OPSLIMIT_MODERATE,
		s.crypto_pwhash_MEMLIMIT_MODERATE,
		s.crypto_pwhash_ALG_DEFAULT
	);
}

export async function randomSalt() {
	const s = await ready();
	return s.randombytes_buf(s.crypto_pwhash_SALTBYTES);
}

/** XChaCha20-Poly1305 uses a 24-byte nonce, fresh for every chunk. */
export async function randomNonce() {
	const s = await ready();
	return s.randombytes_buf(s.crypto_aead_xchacha20poly1305_ietf_NPUBBYTES);
}

export async function encryptChunk(plaintext, key, nonce) {
	const s = await ready();
	return s.crypto_aead_xchacha20poly1305_ietf_encrypt(
		new Uint8Array(plaintext),
		null,
		null,
		new Uint8Array(nonce),
		new Uint8Array(key)
	);
}

export async function decryptChunk(ciphertext, key, nonce) {
	const s = await ready();
	return s.crypto_aead_xchacha20poly1305_ietf_decrypt(
		null,
		new Uint8Array(ciphertext),
		null,
		new Uint8Array(nonce),
		new Uint8Array(key)
	);
}

/** SHA-256 of the whole plaintext, hex. The server stores it as an integrity marker. */
export async function sha256Hex(bytes) {
	const s = await ready();
	return Array.from(s.crypto_hash_sha256(new Uint8Array(bytes)))
		.map((b) => b.toString(16).padStart(2, '0'))
		.join('');
}

export const b64 = {
	encode: (bytes) => Buffer.from(bytes).toString('base64'),
	decode: (str) => new Uint8Array(Buffer.from(str, 'base64'))
};
