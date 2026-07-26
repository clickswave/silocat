import { createToken } from '$lib/jwt.js';
import { dev } from '$app/environment';

const SESSION_COOKIE = {
	name: 'silocat-session',
	age: 60 * 60 * 24 * 30,
	path: '/'
};


let createSession = (sessionData) => {
	let token = createToken(sessionData);
	return {
		name: SESSION_COOKIE.name,
		value: token,
		options: {
			path: SESSION_COOKIE.path,
			httpOnly: true,
			sameSite: 'lax',
			// Secure everywhere except local dev (http://localhost). The cookie
			// carries the session JWT (incl. the api_key): never send it cleartext.
			secure: !dev,
			maxAge: SESSION_COOKIE.age
		}
	};
};


export { createSession, SESSION_COOKIE };
