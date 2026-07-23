import jwt from 'jsonwebtoken';
import { env } from '$env/dynamic/private';
const { JWT_SECRET } = env;


let createToken = (tokenData) => {
	// Strip the registered claims jsonwebtoken manages itself. When we re-sign a
	// session (profile save, email verify, etc.) the payload was decoded from an
	// existing token and already carries `iat`/`exp`/`nbf`; passing those back
	// alongside `expiresIn` makes jwt.sign throw "the payload already has an exp
	// property". Dropping them lets the fresh 30d expiry apply cleanly.
	const { iat, exp, nbf, ...claims } = tokenData || {};
	return jwt.sign(
		claims,
		JWT_SECRET,
		{ expiresIn: '30d' }
	);
};

let decodeToken = (token) => {
	try {
		return jwt.verify(token, JWT_SECRET);
	} catch{
		return null;
	}
};

export {createToken, decodeToken};
