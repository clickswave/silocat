import { createToken } from '$lib/jwt.js';

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
			secure: false,
			maxAge: SESSION_COOKIE.age
		}
	};
};


export { createSession, SESSION_COOKIE };
