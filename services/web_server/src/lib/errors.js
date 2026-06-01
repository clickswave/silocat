export function sessionError() {
	return {
		status: 401,
		message: 'Unauthorized',
		errors: ['You must be logged in to access this resource'],
		data: {}
	};
}
