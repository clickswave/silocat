import sodium from 'libsodium-wrappers-sumo';

let sodiumReady = false;

async function init() {
    await sodium.ready;
    sodiumReady = true;
    postMessage({ type: 'ready' });
}

init();

self.onmessage = async (e) => {
    if (!sodiumReady) {
        await init();
    }

    const { id, type, payload } = e.data;

    try {
        let result;
        switch (type) {
            case 'hashFile':
                result = await computeFileHash(payload.file, payload.chunkSize);
                break;
            case 'deriveKey':
                result = deriveKey(payload.password, payload.salt);
                break;
            case 'encryptChunk':
                result = encryptChunk(payload.chunk, payload.key, payload.nonce);
                break;
            default:
                throw new Error(`Unknown message type: ${type}`);
        }
        postMessage({ id, type, status: 'success', result });
    } catch (error) {
        postMessage({ id, type, status: 'error', error: error.message });
    }
};

async function computeFileHash(file, chunkSize) {
    const state = sodium.crypto_hash_sha256_init();
    let offset = 0;

    // Check if file is a Blob/File or ArrayBuffer (transferable)
    // If it's a File object from input, we can slice it. 
    // Note: Passing File objects to workers is supported in modern browsers.

    while (offset < file.size) {
        const end = Math.min(offset + chunkSize, file.size);
        const chunk = file.slice(offset, end);
        const buffer = new Uint8Array(await chunk.arrayBuffer());
        sodium.crypto_hash_sha256_update(state, buffer);
        offset = end;

        // Report progress occasionally? 
        // For now, simpler to just await the whole thing, but for huge files progress key might be nice.
    }

    return Array.from(sodium.crypto_hash_sha256_final(state))
        .map((b) => b.toString(16).padStart(2, '0'))
        .join('');
}

function deriveKey(password, salt) {
    // Replicating logic from frontend: 
    // sodium.crypto_pwhash(32, password, salt, sodium.crypto_pwhash_OPSLIMIT_INTERACTIVE, sodium.crypto_pwhash_MEMLIMIT_INTERACTIVE, sodium.crypto_pwhash_ALG_DEFAULT)
    // Ensuring we receive uint8arrays.

    const saltBytes = new Uint8Array(salt); // Assuming passed as array or buffer
    return sodium.crypto_pwhash(
        32,
        password,
        saltBytes,
        sodium.crypto_pwhash_OPSLIMIT_MODERATE,
        sodium.crypto_pwhash_MEMLIMIT_MODERATE,
        sodium.crypto_pwhash_ALG_DEFAULT
    );
}

function encryptChunk(chunk, key, nonce) {
    // chunk: Uint8Array, key: Uint8Array, nonce: Uint8Array/String(base64?)
    // In previous code: encryptChunk(chunkBuffer, key, chunkMeta._rawNonce)
    // _rawNonce was Uint8Array.

    // We need to support transferables for speed, so encryption should return buffer.
    const cipherText = sodium.crypto_aead_xchacha20poly1305_ietf_encrypt(
        new Uint8Array(chunk),
        null, // associated data
        null, // secret nonce
        new Uint8Array(nonce),
        new Uint8Array(key)
    );
    return cipherText; // Uint8Array
}
