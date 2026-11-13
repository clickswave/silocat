// Lets a unit test import a module that imports SvelteKit's $app/navigation,
// which only exists inside a SvelteKit build. Import this file first, then the
// module under test with a dynamic import(). The stub's beforeNavigate keeps
// every callback in globalThis.beforeNavigateCallbacks, for the test to call.
import * as nodeModule from 'node:module';

const STUB =
	'data:text/javascript,' +
	encodeURIComponent(
		'export function beforeNavigate(fn) { (globalThis.beforeNavigateCallbacks ??= []).push(fn); }'
	);

if (typeof nodeModule.registerHooks === 'function') {
	nodeModule.registerHooks({
		resolve(specifier, context, next) {
			if (specifier === '$app/navigation') return { url: STUB, shortCircuit: true };
			return next(specifier, context);
		}
	});
} else {
	// Node 20 has only the hooks that run on a thread of their own.
	const hooks = `export function resolve(specifier, context, next) {
		if (specifier === '$app/navigation') return { url: ${JSON.stringify(STUB)}, shortCircuit: true };
		return next(specifier, context);
	}`;
	nodeModule.register('data:text/javascript,' + encodeURIComponent(hooks));
}
