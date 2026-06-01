import jwt from 'jsonwebtoken';
import { env } from '$env/dynamic/private';
const { JWT_SECRET } = env;


let createToken = (tokenData) => {
	return jwt.sign(
		tokenData,
		JWT_SECRET,
		{}
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
