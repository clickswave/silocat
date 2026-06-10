<script>
	import { onMount } from 'svelte';
	import Icon from '@iconify/svelte';
	import { countries, getCountryName } from '$lib/countries';
	import { enhance } from '$app/forms';
	import { invalidateAll } from '$app/navigation';
	import CountrySelect from '$lib/components/CountrySelect.svelte';
	import { browser } from '$app/environment';

	let { data } = $props();

	const defaultUser = {
		username: 'User',
		email: 'user@example.com',
		country: ''
	};

	let user = $derived(data.user || defaultUser);

	// Username change limit (max 2 per rolling 30 days), surfaced from the load.
	let usernameStatus = $derived(data.usernameStatus);
	let usernameLocked = $derived(!!usernameStatus && usernameStatus.remaining <= 0);
	function fmtDate(iso) {
		try {
			return new Date(iso).toLocaleDateString(undefined, {
				month: 'short',
				day: 'numeric',
				year: 'numeric'
			});
		} catch {
			return '';
		}
	}

	let theme = $state('dark');
	let emailNotifications = $state(true);
	let pushNotifications = $state(false);
	let saving = $state(false);

	// --- Email change (inline verify) ---
	let emailInput = $state('');
	let emailStage = $state('idle'); // 'idle' | 'sent'
	let emailBusy = $state(false);
	let otpInput = $state('');
	let otpBusy = $state(false);
	let resendBusy = $state(false);

	$effect(() => {
		// keep the field in sync with the account email when not mid-change
		if (emailStage === 'idle') emailInput = user.email || '';
	});

	const emailValid = $derived(/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(emailInput.trim()));
	const emailChanged = $derived(
		emailInput.trim().toLowerCase() !== (user.email || '').toLowerCase()
	);

	async function requestEmailChange() {
		if (!emailChanged || !emailValid || emailBusy) return;
		emailBusy = true;
		try {
			const res = await fetch('/api/v1/user/request-email-change', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({ email: emailInput.trim() })
			});
			const d = await res.json();
			if (!res.ok) {
				alert(d.message || (d.errors && d.errors.join(', ')) || d.error || 'Could not send code');
			} else {
				emailStage = 'sent';
				otpInput = '';
			}
		} catch (e) {
			console.error(e);
			alert('Could not send verification code');
		} finally {
			emailBusy = false;
		}
	}

	async function resendEmailCode() {
		if (resendBusy) return;
		resendBusy = true;
		try {
			const res = await fetch('/api/v1/user/request-email-change', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({ email: emailInput.trim() })
			});
			const d = await res.json();
			if (!res.ok) alert(d.message || d.error || 'Could not resend code');
		} catch (e) {
			console.error(e);
		} finally {
			resendBusy = false;
		}
	}

	async function confirmEmailChange() {
		if (otpBusy || otpInput.trim().length < 4) return;
		otpBusy = true;
		try {
			const res = await fetch('/api/v1/user/confirm-email-change', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({ otp: otpInput.trim() })
			});
			const d = await res.json();
			if (!res.ok) {
				alert(d.message || (d.errors && d.errors.join(', ')) || d.error || 'Incorrect code');
			} else {
				emailStage = 'idle';
				otpInput = '';
				await invalidateAll();
			}
		} catch (e) {
			console.error(e);
			alert('Could not verify code');
		} finally {
			otpBusy = false;
		}
	}

	function cancelEmailChange() {
		emailStage = 'idle';
		otpInput = '';
		emailInput = user.email || '';
	}

	// Display picture. Works for both Google avatars (absolute http URLs) and
	// uploaded avatars (relative proxy URL with a ?v= cache-buster).
	let fileInput = $state(null);
	let avatarBusy = $state(false);
	let currentAvatar = $state(data.user?.profile_image || null);

	async function onAvatarPick(e) {
		const file = e.target.files?.[0];
		if (!file) return;
		if (file.size > 1024 * 1024) {
			alert('Image must be 1 MB or smaller.');
			e.target.value = '';
			return;
		}
		avatarBusy = true;
		try {
			const res = await fetch('/api/v1/user/avatar', {
				method: 'POST',
				headers: { 'Content-Type': file.type || 'application/octet-stream' },
				body: file
			});
			const result = await res.json();
			if (!res.ok) {
				alert(result.error || 'Upload failed');
			} else {
				currentAvatar = result.profile_image;
			}
		} catch (err) {
			console.error(err);
			alert('Upload failed');
		} finally {
			avatarBusy = false;
			if (e.target) e.target.value = '';
		}
	}

	async function removeAvatar() {
		if (avatarBusy) return;
		avatarBusy = true;
		try {
			const res = await fetch('/api/v1/user/avatar', { method: 'DELETE' });
			if (res.ok) {
				currentAvatar = null;
			} else {
				alert('Failed to remove display picture');
			}
		} catch (err) {
			console.error(err);
			alert('Failed to remove display picture');
		} finally {
			avatarBusy = false;
		}
	}

	let profileForm = $state({
		username: user.username,
		email: user.email,
		bio: user.bio || '',
		country: user.country || ''
	});

	onMount(() => {
		if (browser) {
			const savedTheme = localStorage.getItem('theme') || 'dark';
			setTheme(savedTheme);
		}
	});

	function setTheme(newTheme) {
		theme = newTheme;
		if (browser) {
			document.documentElement.setAttribute('data-theme', newTheme);
			localStorage.setItem('theme', newTheme);
		}
	}

	$effect(() => {
		profileForm.username = user.username;
		profileForm.email = user.email;
		profileForm.bio = user.bio || '';
		profileForm.country = user.country || '';
	});

	// Use enhance for progressive enhancement handling
	const handleSubmit = () => {
		saving = true;
		return async ({ result, update }) => {
			if (result.type === 'success') {
				// SvelteKit automatically invalidates load functions
				await update({ reset: false });
			} else if (result.type === 'failure') {
				alert(result.data?.message || 'Failed to update profile');
			} else {
				alert('An error occurred');
			}
			saving = false;
		};
	};

	// Password: "set" for Google accounts without one, "change" otherwise.
	let hasPassword = $state(data.user?.password_set ?? true);
	let pwForm = $state({ current: '', next: '', confirm: '' });
	let pwBusy = $state(false);

	async function submitPassword(e) {
		e?.preventDefault?.();
		if (pwForm.next !== pwForm.confirm) {
			alert('New passwords do not match');
			return;
		}
		if (hasPassword && !pwForm.current) {
			alert('Enter your current password');
			return;
		}
		pwBusy = true;
		try {
			const body = { new_password: pwForm.next };
			if (hasPassword) body.current_password = pwForm.current;
			const res = await fetch('/api/v1/user/change-password', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify(body)
			});
			const d = await res.json();
			if (!res.ok) {
				alert(d.error || (d.errors && d.errors.join(', ')) || 'Could not update password');
			} else {
				alert(hasPassword ? 'Password updated' : 'Password set');
				pwForm = { current: '', next: '', confirm: '' };
				hasPassword = true;
			}
		} catch (err) {
			console.error(err);
			alert('Could not update password');
		} finally {
			pwBusy = false;
		}
	}
</script>

<div class="settings-page">
	<header>
		<h1>Settings</h1>
		<p class="subtitle">Manage your account preferences and profile.</p>
	</header>

	<div class="settings-grid">
		<!-- Section: Platform Settings -->
		<section class="card platform-settings">
			<div class="card-header">
				<div class="header-icon">
					<Icon icon="ri:macbook-line" width="24" />
				</div>
				<h2>Platform Settings</h2>
			</div>

			<div class="setting-item">
				<div class="label">
					<span class="title">Theme</span>
					<span class="desc">Customize the look and feel of SiloCat.</span>
				</div>
				<div class="controls">
					<button
						class="toggle-btn {theme === 'dark' ? 'active' : ''}"
						onclick={() => setTheme('dark')}
					>
						<Icon icon="ri:moon-line" /> Dark
					</button>
					<button
						class="toggle-btn {theme === 'light' ? 'active' : ''}"
						onclick={() => setTheme('light')}
					>
						<Icon icon="ri:sun-line" /> Light
					</button>
				</div>
			</div>

			<div class="setting-item">
				<div class="label">
					<span class="title">Email Notifications</span>
					<span class="desc">Receive emails about your account activity.</span>
				</div>
				<label class="switch">
					<input
						type="checkbox"
						checked={emailNotifications}
						onchange={(e) => (emailNotifications = e.target.checked)}
					/>
					<span class="slider round"></span>
				</label>
			</div>

			<div class="setting-item">
				<div class="label">
					<span class="title">Push Notifications</span>
					<span class="desc">Receive push notifications on your device.</span>
				</div>
				<label class="switch">
					<input
						type="checkbox"
						checked={pushNotifications}
						onchange={(e) => (pushNotifications = e.target.checked)}
					/>
					<span class="slider round"></span>
				</label>
			</div>
		</section>

		<!-- Section: Profile Settings -->
		<section class="card profile-settings">
			<div class="card-header">
				<div class="header-icon">
					<Icon icon="ri:user-settings-line" width="24" />
				</div>
				<h2>Profile Settings</h2>
			</div>

			<div class="profile-header">
				<div class="avatar-wrapper">
					{#if currentAvatar}
						<img src={currentAvatar} alt="avatar" referrerpolicy="no-referrer" />
					{:else}
						<div class="avatar-placeholder">
							<Icon icon="ri:user-smile-line" width="40" />
						</div>
					{/if}
					<button
						class="edit-avatar"
						title="Change display picture"
						onclick={() => fileInput?.click()}
						disabled={avatarBusy}
					>
						<Icon icon={avatarBusy ? 'svg-spinners:ring-resize' : 'ri:camera-line'} />
					</button>
					<input
						bind:this={fileInput}
						type="file"
						accept="image/png,image/jpeg,image/webp,image/gif"
						class="avatar-input"
						onchange={onAvatarPick}
					/>
				</div>
				<div class="user-meta">
					<h3>{user.username}</h3>
					<span class="role">Pro Member</span>
					<span class="country-badge" title="Country">
						{getCountryName(user.country)}
					</span>
					{#if currentAvatar}
						<button class="remove-avatar" onclick={removeAvatar} disabled={avatarBusy}>
							Remove picture
						</button>
					{/if}
				</div>
			</div>

			<form class="profile-form" method="POST" action="?/saveProfile" use:enhance={handleSubmit}>
				<div class="form-group">
					<label for="username">Username</label>
					<input
						type="text"
						id="username"
						name="username"
						bind:value={profileForm.username}
						placeholder="Enter username"
						minlength="3"
						maxlength="30"
						autocomplete="off"
						disabled={usernameLocked}
					/>
					{#if usernameStatus && usernameStatus.remaining > 0}
						<span class="note">
							You can change your username {usernameStatus.remaining}
							more time{usernameStatus.remaining === 1 ? '' : 's'} this month.
						</span>
					{:else if usernameLocked}
						<span class="note warn">
							You've used both username changes for this month.{usernameStatus.next_change_at
								? ` You can change it again on ${fmtDate(usernameStatus.next_change_at)}.`
								: ''}
						</span>
					{:else}
						<span class="note">You can change your username up to twice a month.</span>
					{/if}
				</div>

				<div class="form-group">
					<label for="country">Country</label>
					<input type="hidden" name="country" value={profileForm.country} />
					<CountrySelect bind:value={profileForm.country} />
				</div>

				<div class="form-group">
					<label for="bio">Bio</label>
					<textarea
						id="bio"
						name="bio"
						bind:value={profileForm.bio}
						placeholder="Tell us about yourself"
						rows="3"
					></textarea>
				</div>

				<div class="form-actions">
					<button type="submit" class="btn primary" disabled={saving}>
						{saving ? 'Saving...' : 'Save Changes'}
					</button>
				</div>
			</form>

			<!-- Email change (inline verify) -->
			<div class="email-change">
				<label for="email">Email Address</label>
				<div class="email-row">
					<input
						type="email"
						id="email"
						bind:value={emailInput}
						placeholder="you@example.com"
						autocomplete="email"
						disabled={emailStage === 'sent'}
					/>
					{#if emailChanged && emailStage === 'idle'}
						<button
							type="button"
							class="btn primary sm"
							onclick={requestEmailChange}
							disabled={emailBusy || !emailValid}
						>
							{emailBusy ? 'Sending…' : 'Verify'}
						</button>
					{/if}
				</div>

				{#if emailStage === 'idle'}
					{#if user.email_verified === false}
						<span class="note warn">Your email is not verified.</span>
					{:else}
						<span class="note">Change your email and click Verify to confirm it with a code.</span>
					{/if}
				{:else}
					<div class="otp-box">
						<p class="otp-info">
							Enter the 6-digit code we sent to <strong>{emailInput}</strong>.
						</p>
						<div class="otp-row">
							<input
								type="text"
								inputmode="numeric"
								maxlength="6"
								placeholder="000000"
								class="otp-input"
								bind:value={otpInput}
							/>
							<button
								type="button"
								class="btn primary sm"
								onclick={confirmEmailChange}
								disabled={otpBusy || otpInput.trim().length < 4}
							>
								{otpBusy ? 'Verifying…' : 'Confirm'}
							</button>
						</div>
						<div class="otp-actions">
							<button type="button" class="link-btn" onclick={resendEmailCode} disabled={resendBusy}>
								{resendBusy ? 'Resending…' : 'Resend code'}
							</button>
							<button type="button" class="link-btn" onclick={cancelEmailChange}>Cancel</button>
						</div>
					</div>
				{/if}
			</div>
		</section>

		<!-- Section: Security -->
		<section class="card security-settings">
			<div class="card-header">
				<div class="header-icon">
					<Icon icon="ri:lock-2-line" width="24" />
				</div>
				<h2>Security</h2>
			</div>

			<form class="profile-form" onsubmit={submitPassword}>
				{#if hasPassword}
					<div class="form-group">
						<label for="current-password">Current password</label>
						<input
							type="password"
							id="current-password"
							bind:value={pwForm.current}
							placeholder="••••••••"
							autocomplete="current-password"
						/>
					</div>
				{:else}
					<p class="note">
						You signed up with Google and don't have a password yet. Set one to also sign in with
						email and password.
					</p>
				{/if}

				<div class="form-group">
					<label for="new-password">{hasPassword ? 'New password' : 'Password'}</label>
					<input
						type="password"
						id="new-password"
						bind:value={pwForm.next}
						placeholder="••••••••"
						autocomplete="new-password"
					/>
				</div>

				<div class="form-group">
					<label for="confirm-password">Confirm password</label>
					<input
						type="password"
						id="confirm-password"
						bind:value={pwForm.confirm}
						placeholder="••••••••"
						autocomplete="new-password"
					/>
				</div>

				<div class="form-actions">
					<button type="submit" class="btn primary" disabled={pwBusy}>
						{pwBusy ? 'Saving...' : hasPassword ? 'Update password' : 'Set password'}
					</button>
				</div>
			</form>
		</section>
	</div>
</div>

<style lang="scss">
	.settings-page {
		width: 100%;
		padding-bottom: var(--space-10);

		header {
			margin-bottom: var(--space-6);
			h1 {
				font-size: var(--fs-h3);
				font-weight: var(--fw-semibold);
				margin-bottom: var(--space-1);
				color: var(--text-primary);
			}
			.subtitle {
				color: var(--text-muted);
				font-size: var(--fs-sm);
			}
		}
	}

	.settings-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(400px, 1fr));
		gap: var(--space-6);
	}

	.card {
		background: var(--bg-card);
		border: 1px solid var(--border-default);
		border-radius: var(--radius-md);
		box-shadow: var(--shadow-card);
		padding: var(--space-5);
		display: flex;
		flex-direction: column;
		gap: var(--space-5);

		.card-header {
			display: flex;
			align-items: center;
			gap: var(--space-3);
			padding-bottom: var(--space-4);
			border-bottom: 1px solid var(--hairline);

			.header-icon {
				width: 40px;
				height: 40px;
				border-radius: var(--radius-sm);
				background: var(--tint-soft);
				color: var(--primary);
				display: flex;
				align-items: center;
				justify-content: center;
			}
			h2 {
				font-size: var(--fs-lg);
				font-weight: var(--fw-semibold);
				margin: 0;
				color: var(--text-primary);
			}
		}
	}

	/* Platform Settings Styles */
	.setting-item {
		display: flex;
		justify-content: space-between;
		align-items: center;
		gap: var(--space-4);
		padding: var(--space-2) 0;

		.label {
			display: flex;
			flex-direction: column;
			gap: var(--space-1);
			.title {
				font-weight: var(--fw-medium);
				color: var(--text-primary);
			}
			.desc {
				font-size: var(--fs-sm);
				color: var(--text-muted);
			}
		}

		.controls {
			display: flex;
			gap: var(--space-2);
			background: var(--bg-input);
			padding: var(--space-1);
			border-radius: var(--radius-sm);
			flex-shrink: 0;

			.toggle-btn {
				background: transparent;
				border: none;
				color: var(--text-secondary);
				padding: var(--space-2) var(--space-3);
				border-radius: var(--radius-sm);
				cursor: pointer;
				display: flex;
				align-items: center;
				gap: var(--space-2);
				font-size: var(--fs-sm);
				transition: background var(--dur) var(--ease), color var(--dur) var(--ease);

				&.active {
					background: var(--bg-card);
					color: var(--text-primary);
					box-shadow: var(--shadow-card);
				}
			}
		}
	}

	/* Switch Styles */
	.switch {
		position: relative;
		display: inline-block;
		width: 44px;
		height: 24px;
		flex-shrink: 0;

		input {
			opacity: 0;
			width: 0;
			height: 0;
		}

		.slider {
			position: absolute;
			cursor: pointer;
			top: 0;
			left: 0;
			right: 0;
			bottom: 0;
			background-color: var(--bg-input);
			transition: background 0.3s var(--ease);
			border: 1px solid var(--border-default);

			&.round {
				border-radius: var(--radius-pill);
			}
			&.round:before {
				border-radius: 50%;
			}

			&:before {
				position: absolute;
				content: '';
				height: 16px;
				width: 16px;
				left: 3px;
				bottom: 3px;
				background-color: #fff;
				transition: transform 0.3s var(--ease);
			}
		}

		input:checked + .slider {
			background-color: var(--primary);
			border-color: var(--primary);
		}

		input:checked + .slider:before {
			transform: translateX(20px);
		}
	}

	/* Profile Settings Styles */
	.profile-header {
		display: flex;
		align-items: center;
		gap: var(--space-5);
		margin-bottom: var(--space-2);

		.avatar-wrapper {
			position: relative;
			width: 80px;
			height: 80px;
			flex-shrink: 0;

			img,
			.avatar-placeholder {
				width: 100%;
				height: 100%;
				border-radius: 50%;
				object-fit: cover;
			}
			.avatar-placeholder {
				background: var(--bg-input);
				display: flex;
				align-items: center;
				justify-content: center;
				color: var(--text-muted);
			}

			.edit-avatar {
				position: absolute;
				bottom: 0;
				right: 0;
				background: var(--primary);
				border: 2px solid var(--bg-card);
				color: #fff;
				border-radius: 50%;
				width: 28px;
				height: 28px;
				display: flex;
				align-items: center;
				justify-content: center;
				cursor: pointer;
				&:hover {
					transform: scale(1.1);
				}
			}
		}

		.user-meta {
			h3 {
				margin: 0 0 var(--space-1) 0;
				font-size: var(--fs-h3);
			}
			.role {
				background: var(--tint-soft);
				color: var(--primary);
				padding: 2px 8px;
				border-radius: var(--radius-sm);
				font-size: var(--fs-xs);
				font-weight: var(--fw-semibold);
				text-transform: uppercase;
			}
			.country-badge {
				color: var(--text-muted);
				font-size: var(--fs-sm);
				margin-left: var(--space-2);
			}
		}
	}

	.profile-form {
		display: flex;
		flex-direction: column;
		gap: var(--space-4);

		.form-group {
			display: flex;
			flex-direction: column;
			gap: var(--space-2);

			label {
				font-size: var(--fs-sm);
				font-weight: var(--fw-medium);
				color: var(--text-secondary);
			}

			input,
			textarea {
				background: var(--bg-input);
				border: 1px solid var(--border-default);
				border-radius: var(--radius-sm);
				padding: 0.75rem 0.95rem;
				color: var(--text-primary);
				font-family: inherit;
				font-size: var(--fs-body);
				transition: border-color var(--dur) var(--ease), box-shadow var(--dur) var(--ease);

				&:focus {
					outline: none;
					border-color: var(--primary);
					box-shadow: 0 0 0 3px var(--primary-glow);
				}

				&:disabled {
					opacity: 0.6;
					cursor: not-allowed;
				}
			}

			.note {
				font-size: var(--fs-xs);
				color: var(--text-muted);
				font-style: italic;

				&.warn {
					color: var(--warning, #e0a800);
					font-style: normal;
				}
			}
		}

		.form-actions {
			display: flex;
			justify-content: flex-end;
			gap: var(--space-3);
			margin-top: var(--space-2);
		}
	}

	.btn {
		padding: 0.7rem 1.25rem;
		border-radius: var(--radius-pill);
		font-weight: var(--fw-semibold);
		cursor: pointer;
		border: 1px solid transparent;
		font-size: var(--fs-sm);
		transition: filter var(--dur) var(--ease), background var(--dur) var(--ease),
			border-color var(--dur) var(--ease);

		&.primary {
			background: var(--accent-gradient);
			color: #fff;
			box-shadow: 0 6px 20px -6px var(--primary-glow);
			&:hover:not(:disabled) {
				filter: brightness(1.06);
			}
			&:disabled {
				opacity: 0.55;
				cursor: not-allowed;
			}
		}
		&.secondary {
			background: var(--tint-soft);
			border-color: var(--border-default);
			color: var(--text-primary);
			&:hover {
				background: var(--tint-softer);
				border-color: var(--border-strong);
			}
		}
	}

	/* Email change (inline verify) */
	.email-change {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		margin-top: var(--space-4);
		padding-top: var(--space-4);
		border-top: 1px solid var(--hairline);

		> label {
			font-size: var(--fs-sm);
			font-weight: var(--fw-medium);
			color: var(--text-secondary);
		}
		.email-row,
		.otp-row {
			display: flex;
			gap: var(--space-2);
		}
		input {
			flex: 1;
			min-width: 0;
			background: var(--bg-input);
			border: 1px solid var(--border-default);
			border-radius: var(--radius-sm);
			padding: 0.75rem 0.95rem;
			color: var(--text-primary);
			font-family: inherit;
			font-size: var(--fs-body);
			transition: border-color var(--dur) var(--ease), box-shadow var(--dur) var(--ease);
			&:focus {
				outline: none;
				border-color: var(--primary);
				box-shadow: 0 0 0 3px var(--primary-glow);
			}
			&:disabled {
				opacity: 0.6;
			}
		}
		.otp-input {
			font-family: var(--font-mono);
			letter-spacing: 0.3em;
			text-align: center;
			max-width: 160px;
		}
		.note {
			font-size: var(--fs-xs);
			color: var(--text-muted);
			font-style: italic;
			&.warn {
				color: var(--warning, #e0a800);
				font-style: normal;
			}
		}
		.otp-box {
			display: flex;
			flex-direction: column;
			gap: var(--space-3);
			background: var(--tint-soft);
			border: 1px solid var(--border-default);
			border-radius: var(--radius-sm);
			padding: var(--space-4);
			margin-top: var(--space-1);
			.otp-info {
				margin: 0;
				font-size: var(--fs-sm);
				color: var(--text-secondary);
				strong {
					color: var(--text-primary);
				}
			}
			.otp-actions {
				display: flex;
				gap: var(--space-4);
			}
			.link-btn {
				background: none;
				border: none;
				padding: 0;
				color: var(--primary);
				font-size: var(--fs-sm);
				font-weight: var(--fw-medium);
				cursor: pointer;
				&:hover {
					text-decoration: underline;
				}
				&:disabled {
					opacity: 0.6;
					cursor: default;
				}
			}
		}
	}
	.btn.sm {
		padding: 0.6rem 1.1rem;
		flex-shrink: 0;
	}

	@media (max-width: 600px) {
		.settings-grid {
			grid-template-columns: 1fr;
		}
	}

	/* Display-picture controls */
	.avatar-input {
		display: none;
	}
	.edit-avatar:disabled {
		opacity: 0.7;
		cursor: default;
	}
	.remove-avatar {
		display: block;
		margin-top: var(--space-2);
		background: none;
		border: none;
		padding: 0;
		color: var(--text-muted);
		font-size: var(--fs-xs);
		cursor: pointer;
		text-decoration: underline;
	}
	.remove-avatar:hover {
		color: var(--primary);
	}
	.remove-avatar:disabled {
		opacity: 0.6;
		cursor: default;
	}
</style>
