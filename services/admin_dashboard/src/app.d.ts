// See https://svelte.dev/docs/kit/types#app.d.ts
// for information about these interfaces
declare global {
	namespace App {
		interface Platform {
			env: Env;
			ctx: ExecutionContext;
			caches: CacheStorage;
			cf?: IncomingRequestCfProperties;
		}

		// interface Error {}
		interface Locals {
			session: {
				user: {
					get: () => Promise<any>;
					set: (data: any) => Promise<boolean>;
					update: (data: { key: string; value: any }) => Promise<boolean>;
				};
				subscription: {
					get: () => Promise<any>;
					set: (data: any) => Promise<boolean>;
				};
				get: () => Promise<any>;
				set: (data: any) => Promise<boolean>;
				delete: () => Promise<boolean>;
			};
		}
		// interface PageData {}
		// interface PageState {}
		// interface Platform {}
	}
}

export { };
