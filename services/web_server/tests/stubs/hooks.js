// Lets download.js and the npm client load under plain Node.
//
//   * $lib/cryptoClient.js and $lib/fileSink.js become the stubs next to this file;
//     other $lib/ imports resolve to src/lib.
//   * $app/navigation becomes a no-op beforeNavigate.
//   * packages/client has no node_modules of its own, so its libsodium import
//     resolves from this package instead.
import * as nodeModule from 'node:module';

const here = new URL('./', import.meta.url).href;
const lib = new URL('../../src/lib/', import.meta.url).href;
const webPackage = new URL('../../package.json', import.meta.url).href;
const navigation =
	'data:text/javascript,' + encodeURIComponent('export function beforeNavigate() {}');

const hooks = `
const here = ${JSON.stringify(here)};
const lib = ${JSON.stringify(lib)};
const webPackage = ${JSON.stringify(webPackage)};
const navigation = ${JSON.stringify(navigation)};
export function resolve(specifier, context, next) {
	if (specifier === '$app/navigation') return { url: navigation, shortCircuit: true };
	if (specifier === '$lib/cryptoClient.js') return { url: here + 'cryptoClient.js', shortCircuit: true };
	if (specifier === '$lib/fileSink.js') return { url: here + 'fileSink.js', shortCircuit: true };
	if (specifier.startsWith('$lib/')) return { url: lib + specifier.slice(5), shortCircuit: true };
	if (specifier === 'libsodium-wrappers-sumo' && context.parentURL?.includes('/packages/client/')) {
		return next(specifier, { ...context, parentURL: webPackage });
	}
	return next(specifier, context);
}`;

nodeModule.register('data:text/javascript,' + encodeURIComponent(hooks));
