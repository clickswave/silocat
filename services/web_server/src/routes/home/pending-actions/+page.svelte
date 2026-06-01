<script>
	import { FrontendClient } from '$lib/frontendClient.js';
	import { goto } from '$app/navigation';
	import { toast } from 'svelte-sonner';
	import Icon from '@iconify/svelte';

	let { data } = $props();
	let otp = $state('');
	let processing = $state(false);

	let resending = $state(false);

	async function verifyEmail() {
		if (!otp || processing) return;
		processing = true;

		try {
			const res = await FrontendClient.post('/api/v1/user/verify-email', { otp });
			if (res.data.success) {
				toast.success('Email verified successfully!');
				setTimeout(() => {
					goto('/home');
				}, 1000);
			} else {
				toast.error(res.data.error || 'Verification failed');
			}
		} catch (e) {
			let msg = e.response?.data?.message || e.response?.data?.error || 'Failed to verify email';
			if (e.response?.data?.errors?.length > 0) {
				msg = e.response.data.errors[0];
			}
			console.error(e);
			toast.error(msg);
		} finally {
			processing = false;
		}
	}

	async function resendCode() {
		if (resending) return;
		resending = true;

		try {
			const res = await FrontendClient.post('/api/v1/user/resend-verification', {});
			if (res.data.success || res.status == 200) {
				toast.success('Verification code resent!');
			} else {
				toast.error(res.data.error.message || 'Failed to resend code', {
					description: res.data.errors
				});
			}
			resending = false;
		} catch (e) {
			console.error('Resend error:', e);
			if (e.response?.status == 429) {
				const retryAfter = e.response.data?.data?.retry_after || 60;
				toast.error(`Please wait ${retryAfter} seconds`);
			} else {
				let msg = e.response?.data?.message || e.response?.data?.error || 'Failed to resend code';
				toast.error(msg, {
					description: e?.response?.data?.errors[0]
						? e?.response?.data?.errors.join('\n')
						: 'Please wait atleast 30 seconds'
				});
			}
			resending = false;
		}
	}
</script>

<div class="pending-page">
	<div class="card">
		<div class="icon-circle">
			<Icon icon="ri:mail-check-line" width="40" />
		</div>
		<h1>Check your inbox</h1>
		<p class="desc">
			We sent a verification code to <strong>{data.user?.email}</strong>.<br />
			Please enter the code to verify your account.
		</p>

		<div class="form-group">
			<input type="text" bind:value={otp} placeholder="000000" maxlength="6" class="otp-input" />
			<button class="verify-btn" onclick={verifyEmail} disabled={processing || !otp}>
				{processing ? 'Verifying...' : 'Verify Email'}
			</button>
		</div>

		<p class="footer">
			Didn't receive the email?
			<button class="resend-link" onclick={resendCode} disabled={resending}>
				{resending ? 'Resending...' : 'Resend Code'}
			</button>
		</p>
	</div>
</div>

<style lang="scss">
	.pending-page {
		display: flex;
		align-items: center;
		justify-content: center;
		min-height: 100vh;
		background: #09090b;
		color: white;
		padding: 20px;
	}

	.card {
		background: #18181b;
		border: 1px solid rgba(255, 255, 255, 0.1);
		padding: 3rem;
		border-radius: 20px;
		text-align: center;
		max-width: 450px;
		width: 100%;
		box-shadow: 0 10px 40px rgba(0, 0, 0, 0.5);

		.icon-circle {
			width: 80px;
			height: 80px;
			background: rgba(255, 70, 85, 0.1);
			color: #ff4655;
			border-radius: 50%;
			display: flex;
			align-items: center;
			justify-content: center;
			margin: 0 auto 1.5rem;
		}

		h1 {
			font-size: 1.75rem;
			margin-bottom: 1rem;
			font-weight: 700;
		}

		.desc {
			color: #a1a1aa;
			line-height: 1.6;
			margin-bottom: 2rem;

			strong {
				color: white;
			}
		}
	}

	.form-group {
		display: flex;
		flex-direction: column;
		gap: 1rem;
		margin-bottom: 2rem;

		.otp-input {
			background: #27272a;
			border: 1px solid #3f3f46;
			padding: 1rem;
			border-radius: 12px;
			color: white;
			font-size: 1.25rem;
			text-align: center;
			letter-spacing: 0.5rem;
			outline: none;
			transition: border-color 0.2s;

			&:focus {
				border-color: #ff4655;
			}
		}

		.verify-btn {
			background: #ff4655;
			color: white;
			border: none;
			padding: 1rem;
			border-radius: 12px;
			font-weight: 600;
			cursor: pointer;
			font-size: 1rem;
			transition: background 0.2s;

			&:hover {
				background: #e03e4b;
			}

			&:disabled {
				opacity: 0.5;
				cursor: not-allowed;
			}
		}
	}

	.footer {
		font-size: 0.9rem;
		color: #a1a1aa;

		.resend-link {
			background: none;
			border: none;
			color: #ff4655;
			cursor: pointer;
			font-weight: 600;
			padding: 0;

			&:hover {
				text-decoration: underline;
			}
		}
	}
</style>
