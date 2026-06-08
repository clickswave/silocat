// Loads Razorpay checkout.js on demand (instead of render-blocking on every
// page). Resolves once window.Razorpay is available; caches the in-flight load.
let promise = null;

export function loadRazorpay() {
	if (typeof window === 'undefined') return Promise.reject(new Error('no window'));
	if (window.Razorpay) return Promise.resolve(window.Razorpay);
	if (promise) return promise;

	promise = new Promise((resolve, reject) => {
		const s = document.createElement('script');
		s.src = 'https://checkout.razorpay.com/v1/checkout.js';
		s.async = true;
		s.onload = () => resolve(window.Razorpay);
		s.onerror = () => {
			promise = null;
			reject(new Error('Failed to load Razorpay'));
		};
		document.head.appendChild(s);
	});
	return promise;
}
