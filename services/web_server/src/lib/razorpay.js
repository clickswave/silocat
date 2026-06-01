import Razorpay from 'razorpay';
import { env } from '$env/dynamic/private';
const { RAZORPAY_ID, RAZORPAY_SECRET } = env;
import { validatePaymentVerification } from 'razorpay/dist/utils/razorpay-utils.js';

// razorpay instance
export const RazorpayInstance = new Razorpay({
	key_id: RAZORPAY_ID, key_secret: RAZORPAY_SECRET
});

export const RazorpayVerifyPayment = async ({ paymentId, orderId, signature }) => {
	return validatePaymentVerification(
		{ 'order_id': orderId, 'payment_id': paymentId },
		signature,
		RAZORPAY_SECRET
	);
};
