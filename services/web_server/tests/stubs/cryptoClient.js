// Stands in for $lib/cryptoClient.js, which drives a Web Worker that Node does
// not have. The cipher calls are the worker's own (XChaCha20-Poly1305, no
// associated data). The KDF is a plain hash instead of Argon2id, because these
// tests are about which chunks get joined, not about the key.
import sodium from 'libsodium-wrappers-sumo';

await sodium.ready;

export async function deriveKey(password, salt) {
	return sodium.crypto_generichash(32, sodium.from_string(password), new Uint8Array(salt));
}

export async function encryptChunk(chunk, key, nonce) {
	return sodium.crypto_aead_xchacha20poly1305_ietf_encrypt(
		new Uint8Array(chunk), null, null, new Uint8Array(nonce), new Uint8Array(key)
	);
}

export async function decryptChunk(chunk, key, nonce) {
	return sodium.crypto_aead_xchacha20poly1305_ietf_decrypt(
		null, new Uint8Array(chunk), null, new Uint8Array(nonce), new Uint8Array(key)
	);
}

export async function hashFile() {
	return 'checksum';
}
