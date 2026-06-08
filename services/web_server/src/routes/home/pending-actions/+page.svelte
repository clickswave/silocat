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
		background: var(--bg-app);
		color: var(--text-primary);
		padding: var(--gutter);
	}

	.card {
		background: var(--bg-elevated);
		border: 1px solid var(--border-default);
		padding: var(--space-8);
		border-radius: var(--radius-lg);
		text-align: center;
		max-width: 450px;
		width: 100%;
		box-shadow: var(--shadow-lg);

		.icon-circle {
			width: 80px;
			height: 80px;
			background: var(--tint-soft);
			color: var(--primary);
			border-radius: 50%;
			display: flex;
			align-items: center;
			justify-content: center;
			margin: 0 auto var(--space-5);
			box-shadow: var(--shadow-glow);
		}

		h1 {
			font-size: var(--fs-h3);
			margin-bottom: var(--space-4);
			font-weight: var(--fw-bold);
		}

		.desc {
			color: var(--text-secondary);
			line-height: var(--lh-normal);
			margin-bottom: var(--space-6);

			strong {
				color: var(--text-primary);
			}
		}
	}

	.form-group {
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
		margin-bottom: var(--space-6);

		.otp-input {
			background: var(--bg-input);
			border: 1px solid var(--border-default);
			padding: var(--space-4);
			border-radius: var(--radius-sm);
			color: var(--text-primary);
			font-size: var(--fs-h3);
			font-family: var(--font-mono);
			text-align: center;
			letter-spacing: 0.5rem;
			outline: none;
			transition: border-color var(--dur) var(--ease), box-shadow var(--dur) var(--ease);

			&:focus {
				border-color: var(--primary);
				box-shadow: 0 0 0 3px var(--primary-glow);
			}
		}

		.verify-btn {
			background: var(--accent-gradient);
			color: #fff;
			border: none;
			padding: 0.95rem;
			border-radius: var(--radius-pill);
			font-weight: var(--fw-semibold);
			cursor: pointer;
			font-size: var(--fs-body);
			box-shadow: 0 6px 20px -6px var(--primary-glow);
			transition: filter var(--dur) var(--ease);

			&:hover:not(:disabled) {
				filter: brightness(1.06);
			}

			&:disabled {
				opacity: 0.55;
				cursor: not-allowed;
			}
		}
	}

	.footer {
		font-size: var(--fs-sm);
		color: var(--text-secondary);

		.resend-link {
			background: none;
			border: none;
			color: var(--primary);
			cursor: pointer;
			font-weight: var(--fw-semibold);
			padding: 0;

			&:hover {
				color: var(--primary-hover);
				text-decoration: underline;
			}
		}
	}
</style>
