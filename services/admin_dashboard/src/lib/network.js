import axios from 'axios';
import { env } from '$env/dynamic/private';
const { AUTHORITY_SIGN, INTERNAL_API_URL } = env;

// USER MANAGEMENT SERVER DOWN ERROR
export const ApiServerError = {
	status: 500,
	message: 'Something went wrong',
	errors: ['Last request was not successful. If the problem persists, please contact support.'],
	data: {}
};

// USER MANAGEMENT
export let ApiServerClient = axios.create({
	baseURL: INTERNAL_API_URL,
	headers: { 'X-Authority-Sign': AUTHORITY_SIGN }
});

export const ApiServerRoutes = {

};
