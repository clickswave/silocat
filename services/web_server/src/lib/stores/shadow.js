import { writable } from 'svelte/store';
import { browser } from '$app/environment';

function getInitialKey() {
    if (!browser) return null;

    let key = localStorage.getItem('shadow_api_key');
    if (!key) {
        // Generate new key if none exists
        key = crypto.randomUUID();
        localStorage.setItem('shadow_api_key', key);
    }
    return key;
}

export const shadowKey = writable(getInitialKey());

export function regenerateShadowKey() {
    if (!browser) return;
    const newKey = crypto.randomUUID();
    localStorage.setItem('shadow_api_key', newKey);
    shadowKey.set(newKey);
    return newKey;
}

export function clearShadowKey() {
    if (!browser) return;
    localStorage.removeItem('shadow_api_key');
    shadowKey.set(null);
}

// Subscribe to changes to keep localStorage in sync if set externally? 
// No, mostly standard methods.
