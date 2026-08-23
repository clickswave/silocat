// See https://svelte.dev/docs/kit/types#app.d.ts
// for information about these interfaces

/** A user as the app passes it around: whatever api_switch returned, plus the
 *  fields hooks.server.js and the layouts add. Deliberately loose, because the
 *  shape comes from the backend and is not owned here. */
interface SessionUser {
	id: string;
	/** The account credential. Present server-side only: the root layout strips
	 *  it before anything reaches the browser. */
	api_key?: string;
	username?: string;
	email?: string;
	email_verified?: boolean;
	default_storage_bytes?: number | string;
	totalAvailableSpace?: number;
	subscription?: Record<string, any> | null;
	[key: string]: any;
}

/** The session helper hooks.server.js hangs off `event.locals`. Declaring it
 *  here is what lets every +server.js and +page.server.js type-check its
 *  `locals.session.user.get()` calls; without it each one reported
 *  "Property 'session' does not exist on type 'Locals'". */
interface AppSession {
	get(): Promise<{ user?: SessionUser; [key: string]: any } | null>;
	set(data: any): Promise<boolean>;
	delete(): Promise<boolean>;
	user: {
		/** The authenticated caller: API key first, then the session cookie. */
		get(): Promise<SessionUser | undefined>;
		set(user: SessionUser): Promise<boolean>;
		update(change: { key: string; value: any }): Promise<boolean>;
	};
	subscription: {
		get(): Promise<any>;
		set(data: any): Promise<boolean>;
	};
}

declare global {
	namespace App {
		interface Locals {
			session: AppSession;
		}

		interface Platform {
			env: Env;
			ctx: ExecutionContext;
			caches: CacheStorage;
			cf?: IncomingRequestCfProperties;
		}

		// interface Error {}
		// interface PageData {}
		// interface PageState {}
	}

	interface Window {
		/** File System Access API. Chromium only, and not in the default TS DOM
		 *  lib, so $lib/fileSink.js feature-detects it at runtime and needs the
		 *  declaration here to type-check that detection. */
		showSaveFilePicker?: (options?: {
			suggestedName?: string;
			types?: Array<{ description?: string; accept: Record<string, string[]> }>;
		}) => Promise<FileSystemFileHandle>;
	}
}

export {};
