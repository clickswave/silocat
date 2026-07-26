<script>
	import { onMount } from 'svelte';
	import Icon from '$lib/ui/Icon.svelte';
	import { countries, getCountryName } from '$lib/countries';
	import { enhance } from '$app/forms';
	import { invalidateAll } from '$app/navigation';
	import CountrySelect from '$lib/components/CountrySelect.svelte';
	import { browser } from '$app/environment';
	import { toast } from '$lib/toast.js';
	import { theme as themeStore, setTheme as applyTheme } from '$lib/theme.js';

	let { data } = $props();

	const defaultUser = {
		username: 'User',
		email: 'user@example.com',
		country: ''
	};

	let user = $derived(data.user || defaultUser);
	let isPro = $derived(!!data.user?.subscription);

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
				toast.error(d.message || (d.errors && d.errors.join(', ')) || d.error || 'Could not send code');
			} else {
				emailStage = 'sent';
				otpInput = '';
			}
		} catch (e) {
			console.error(e);
			toast.error('Could not send verification code');
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
			if (!res.ok) toast.error(d.message || d.error || 'Could not resend code');
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
				toast.error(d.message || (d.errors && d.errors.join(', ')) || d.error || 'Incorrect code');
			} else {
				emailStage = 'idle';
				otpInput = '';
				await invalidateAll();
			}
		} catch (e) {
			console.error(e);
			toast.error('Could not verify code');
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
			toast.error('Image must be 1 MB or smaller.');
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
				toast.error(result.error || 'Upload failed');
			} else {
				currentAvatar = result.profile_image;
			}
		} catch (err) {
			console.error(err);
			toast.error('Upload failed');
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
				toast.error('Failed to remove display picture');
			}
		} catch (err) {
			console.error(err);
			toast.error('Failed to remove display picture');
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
			theme = localStorage.getItem('theme') || 'dark';
		}
	});

	function setTheme(newTheme) {
		theme = newTheme;
		applyTheme(newTheme);
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
				toast.error(result.data?.message || 'Failed to update profile');
			} else {
				toast.error('An error occurred');
			}
			saving = false;
		};
	};

	// Password: "set" for Google accounts without one, "change" otherwise.
	// ---- API key ----------------------------------------------------------
	// One key per account, and it is the credential for the whole programmatic
	// surface. Hidden by default so it is not shoulder-surfed or captured in a
	// screen share while someone is looking at unrelated settings.
	let apiKey = $state(data.user?.api_key || '');
	let apiKeyVisible = $state(false);
	let rotating = $state(false);
	let confirmRotate = $state(false);

	let maskedKey = $derived(
		apiKey ? `${apiKey.slice(0, 8)}${'•'.repeat(Math.max(0, apiKey.length - 12))}${apiKey.slice(-4)}` : ''
	);

	function copyApiKey() {
		if (!apiKey) return;
		navigator.clipboard.writeText(apiKey);
		toast.success('API key copied', 'Treat it like a password.');
	}

	async function rotateApiKey() {
		if (rotating) return;
		rotating = true;
		try {
			const res = await fetch('/api/v1/user/rotate-api-key', { method: 'POST' });
			const body = await res.json();
			if (!res.ok || !body?.success?.api_key) throw new Error(body?.message || 'Failed');
			apiKey = body.success.api_key;
			apiKeyVisible = true;
			confirmRotate = false;
			toast.success('API key rotated', 'The old key stopped working immediately.');
		} catch (e) {
			toast.error('Could not rotate the key', e.message || 'Try again in a moment.');
		} finally {
			rotating = false;
		}
	}

	let hasPassword = $state(data.user?.password_set ?? true);
	let pwForm = $state({ current: '', next: '', confirm: '' });
	let pwBusy = $state(false);

	async function submitPassword(e) {
		e?.preventDefault?.();
		if (pwForm.next !== pwForm.confirm) {
			toast.error('New passwords do not match');
			return;
		}
		if (hasPassword && !pwForm.current) {
			toast.error('Enter your current password');
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
				toast.error(d.error || (d.errors && d.errors.join(', ')) || 'Could not update password');
			} else {
				toast.success(hasPassword ? 'Password updated' : 'Password set');
				pwForm = { current: '', next: '', confirm: '' };
				hasPassword = true;
			}
		} catch (err) {
			console.error(err);
			toast.error('Could not update password');
		} finally {
			pwBusy = false;
		}
	}

	let otpDigits = $derived(otpInput.padEnd(6, ' ').slice(0, 6).split(''));

	function onOtpInput(index, event) {
		const raw = event.target.value.replace(/\D/g, '');
		const chars = otpInput.padEnd(6, ' ').split('');
		if (!raw) {
			chars[index] = ' ';
			otpInput = chars.join('').trimEnd();
			return;
		}
		// Paste of a full code lands in one box; spread it across all six.
		if (raw.length > 1) {
			otpInput = raw.slice(0, 6);
			const last = Math.min(raw.length, 6) - 1;
			document.getElementById(`otp-${last}`)?.focus();
			return;
		}
		chars[index] = raw;
		otpInput = chars.join('').trimEnd();
		if (index < 5) document.getElementById(`otp-${index + 1}`)?.focus();
	}

	function onOtpKeydown(index, event) {
		if (event.key === 'Backspace' && !event.target.value && index > 0) {
			document.getElementById(`otp-${index - 1}`)?.focus();
		}
	}
</script>

<div class="settings">
	<div class="col">
		<header class="head">
			<h1>Settings</h1>
			<span class="sub">Your account, in plain sections.</span>
		</header>

		<!-- Platform -->
		<section class="row-section">
			<div class="row-text">
				<span class="row-title">Theme</span>
				<span class="row-note">Dark by default. Light if you insist.</span>
			</div>
			<div class="seg">
				<button
					type="button"
					class:on={theme === 'dark'}
					onclick={() => setTheme('dark')}
					aria-pressed={theme === 'dark'}
				>
					Dark
				</button>
				<button
					type="button"
					class:on={theme === 'light'}
					onclick={() => setTheme('light')}
					aria-pressed={theme === 'light'}
				>
					Light
				</button>
			</div>
		</section>

		<!-- Profile -->
		<section class="section">
			<span class="section-label">Profile</span>

			<div class="identity">
				<div class="avatar-wrap">
					{#if currentAvatar}
						<img class="avatar" src={currentAvatar} alt="" referrerpolicy="no-referrer" />
					{:else}
						<div class="avatar fallback">
							{(user.username || '?').charAt(0).toUpperCase()}
						</div>
					{/if}
					<button
						type="button"
						class="avatar-btn"
						aria-label="Change picture"
						disabled={avatarBusy}
						onclick={() => fileInput?.click()}
					>
						<Icon name={avatarBusy ? 'spinner' : 'camera'} size={14} />
					</button>
					<input
						bind:this={fileInput}
						type="file"
						accept="image/png,image/jpeg,image/webp,image/gif"
						hidden
						onchange={onAvatarPick}
					/>
				</div>

				<div class="identity-text">
					<span class="identity-name">{user.username}</span>
					<div class="chips">
						<span class="chip">{isPro ? 'Pro' : 'Free'}</span>
						{#if user.country}
							<span class="chip">{getCountryName(user.country)}</span>
						{/if}
						{#if currentAvatar}
							<button type="button" class="link-btn" disabled={avatarBusy} onclick={removeAvatar}>
								Remove picture
							</button>
						{/if}
					</div>
				</div>
			</div>

			<form class="fields" method="POST" action="?/saveProfile" use:enhance={handleSubmit}>
				<div class="f-field">
					<label for="username">Username</label>
					<input
						type="text"
						id="username"
						name="username"
						bind:value={profileForm.username}
						placeholder="username"
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
							You've used both username changes for this month.{usernameStatus?.next_change_at
								? ` You can change it again on ${fmtDate(usernameStatus.next_change_at)}.`
								: ''}
						</span>
					{:else}
						<span class="note">You can change your username up to twice a month.</span>
					{/if}
				</div>

				<div class="f-field">
					<label for="country">Country</label>
					<input type="hidden" name="country" value={profileForm.country} />
					<CountrySelect bind:value={profileForm.country} />
				</div>

				<div class="f-field">
					<label for="bio">Bio</label>
					<textarea
						id="bio"
						name="bio"
						rows="3"
						placeholder="A line about you, optional."
						bind:value={profileForm.bio}
					></textarea>
				</div>

				<div>
					<button type="submit" class="primary" disabled={saving}>
						{saving ? 'Saving…' : 'Save changes'}
					</button>
				</div>
			</form>
		</section>

		<!-- Email -->
		<section class="section">
			<span class="section-label">Email</span>

			{#if emailStage === 'idle'}
				<div class="f-field">
					<label for="email">Email address</label>
					<div class="inline">
						<input
							type="email"
							id="email"
							bind:value={emailInput}
							placeholder="you@example.com"
							autocomplete="email"
						/>
						{#if emailChanged}
							<button
								type="button"
								class="ghost"
								disabled={emailBusy || !emailValid}
								onclick={requestEmailChange}
							>
								{emailBusy ? 'Sending…' : 'Verify'}
							</button>
						{/if}
					</div>
					{#if user.email_verified === false}
						<span class="note warn">Your email is not verified.</span>
					{:else}
						<span class="note">Change your email and click Verify to confirm it with a code.</span>
					{/if}
				</div>
			{:else}
				<div class="otp-stage">
					<div class="f-field">
						<label for="email-locked">Email address</label>
						<input id="email-locked" type="email" value={emailInput} disabled />
					</div>

					<span class="otp-line">
						Enter the 6-digit code we sent to <strong>{emailInput}</strong>
					</span>

					<div class="otp-boxes">
						{#each otpDigits as digit, i (i)}
							<input
								id={`otp-${i}`}
								type="text"
								inputmode="numeric"
								maxlength="6"
								value={digit.trim()}
								class:filled={digit.trim() !== ''}
								oninput={(e) => onOtpInput(i, e)}
								onkeydown={(e) => onOtpKeydown(i, e)}
								aria-label={`Digit ${i + 1}`}
							/>
						{/each}
					</div>

					<div class="otp-actions">
						<button
							type="button"
							class="primary sm"
							disabled={otpBusy || otpInput.trim().length < 6}
							onclick={confirmEmailChange}
						>
							{otpBusy ? 'Verifying…' : 'Confirm'}
						</button>
						<button type="button" class="link-btn" disabled={resendBusy} onclick={resendEmailCode}>
							{resendBusy ? 'Resending…' : 'Resend code'}
						</button>
						<button type="button" class="link-btn faint" onclick={cancelEmailChange}>Cancel</button>
					</div>
				</div>
			{/if}
		</section>

		<!-- API access -->
		<section class="section">
			<span class="section-label">API</span>

			<div class="f-field">
				<label for="api-key">Your API key</label>
				<div class="key-row">
					<input
						id="api-key"
						class="mono"
						type="text"
						readonly
						value={apiKeyVisible ? apiKey : maskedKey}
						onclick={(e) => e.target.select()}
					/>
					<button
						type="button"
						class="ghost icon"
						aria-label={apiKeyVisible ? 'Hide API key' : 'Show API key'}
						title={apiKeyVisible ? 'Hide' : 'Show'}
						onclick={() => (apiKeyVisible = !apiKeyVisible)}
					>
						<Icon name="eye" size={15} />
					</button>
					<button type="button" class="ghost icon" aria-label="Copy API key" title="Copy" onclick={copyApiKey}>
						<Icon name="copy" size={15} />
					</button>
				</div>
				<span class="note">
					Send it as the <code>X-Api-Key</code> header to authenticate against
					<a href="/api">the API</a>. It identifies your account, so treat it like a
					password: anything holding it can read, share and delete your files.
				</span>
			</div>

			{#if confirmRotate}
				<div class="danger-box">
					<span class="danger-title">Rotate this key?</span>
					<span class="danger-line">
						The current key stops working immediately and there is no overlap window.
						Anything using it, scripts, integrations, another device, breaks until you
						paste the new one in.
					</span>
					<div class="danger-actions">
						<button type="button" class="link-btn" onclick={() => (confirmRotate = false)}>
							Cancel
						</button>
						<button type="button" class="danger-btn" disabled={rotating} onclick={rotateApiKey}>
							{rotating ? 'Rotating…' : 'Rotate key'}
						</button>
					</div>
				</div>
			{:else}
				<div>
					<button type="button" class="ghost" onclick={() => (confirmRotate = true)}>
						Rotate key
					</button>
				</div>
			{/if}
		</section>

		<!-- Security -->
		<section class="section last">
			<span class="section-label">Security</span>

			{#if !hasPassword}
				<p class="note block">
					You signed up with Google and don't have a password yet. Set one to also sign in with
					email and password.
				</p>
			{/if}

			<form class="fields" onsubmit={submitPassword}>
				{#if hasPassword}
					<div class="f-field">
						<label for="current-password">Current password</label>
						<input
							type="password"
							id="current-password"
							class="mono"
							bind:value={pwForm.current}
							autocomplete="current-password"
						/>
					</div>
				{/if}

				<div class="pair">
					<div class="f-field">
						<label for="new-password">{hasPassword ? 'New password' : 'Password'}</label>
						<input
							type="password"
							id="new-password"
							class="mono"
							bind:value={pwForm.next}
							autocomplete="new-password"
						/>
					</div>
					<div class="f-field">
						<label for="confirm-password">Confirm password</label>
						<input
							type="password"
							id="confirm-password"
							class="mono"
							bind:value={pwForm.confirm}
							autocomplete="new-password"
						/>
					</div>
				</div>

				<div>
					<button type="submit" class="primary" disabled={pwBusy}>
						{pwBusy ? 'Saving…' : hasPassword ? 'Update password' : 'Set password'}
					</button>
				</div>
			</form>
		</section>
	</div>
</div>

<style lang="scss">
	.settings {
		padding-bottom: var(--space-6);
	}

	/* Sparse, hairline-separated sections. No cards inside cards.
	   The section label sits in its own left column so the page reads as
	   label/content pairs and fills the pane, instead of a narrow strip of
	   fields against a wide void. Form controls keep a readable measure. */
	.col {
		/* Full width of the content pane, like every other app screen. The label
		   rail plus a capped control column keeps fields at a readable measure
		   without leaving a void on the right. */
		width: 100%;
		display: flex;
		flex-direction: column;
	}

	.head {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
		padding: var(--space-2) 0.125rem var(--space-5);

		h1 {
			margin: 0;
			font-size: var(--fs-h2);
			font-weight: var(--fw-black);
			letter-spacing: var(--tracking-tight);
			line-height: var(--lh-tight);
		}
	}

	.sub {
		font-size: var(--fs-sm);
		color: var(--ink-faint);
	}

	.row-section {
		display: grid;
		grid-template-columns: 220px minmax(0, 720px);
		column-gap: 3.5rem;
		align-items: center;
		padding: 1.5rem 0.125rem;
		border-top: 1px solid var(--edge);
	}

	.row-text {
		display: flex;
		flex-direction: column;
		gap: 0.125rem;
	}

	.row-title {
		font-size: 0.875rem;
		font-weight: var(--fw-medium);
	}

	.row-note {
		font-size: var(--fs-sm);
		color: var(--ink-faint);
	}

	.seg {
		display: flex;
		padding: 2px;
		border-radius: 8px;
		background: var(--tint-soft);
		border: 1px solid var(--edge);

		button {
			height: 28px;
			padding-inline: 0.875rem;
			border-radius: var(--radius-sm);
			border: 1px solid transparent;
			background: transparent;
			font: inherit;
			font-size: var(--fs-sm);
			font-weight: var(--fw-medium);
			color: var(--ink-faint);
			cursor: pointer;
			transition:
				background var(--dur-fast) var(--ease),
				color var(--dur-fast) var(--ease);

			&.on {
				background: var(--raised);
				border-color: var(--edge);
				color: var(--ink);
			}
		}
	}

	.section {
		display: grid;
		grid-template-columns: 220px minmax(0, 720px);
		column-gap: 3.5rem;
		row-gap: 1.25rem;
		align-items: start;
		padding: 1.5rem 0.125rem;
		border-top: 1px solid var(--edge);

		/* The label owns column one; everything else stacks in column two. */
		> :not(.section-label) {
			grid-column: 2;
		}

		&.last {
			padding-bottom: var(--space-6);
		}
	}

	.section-label {
		padding-top: 0.375rem;
		font-size: var(--fs-xs);
		font-weight: var(--fw-medium);
		color: var(--ink-faint);
		text-transform: uppercase;
		letter-spacing: 0.06em;
	}

	/* ---- identity ---- */
	.identity {
		display: flex;
		align-items: center;
		gap: 1.25rem;
	}

	.avatar-wrap {
		position: relative;
		flex: 0 0 88px;
	}

	.avatar {
		width: 88px;
		height: 88px;
		border-radius: var(--radius-full);
		object-fit: cover;
		display: block;

		&.fallback {
			background: var(--tint-softer);
			border: 1px solid var(--edge);
			display: grid;
			place-items: center;
			font-size: 1.75rem;
			font-weight: var(--fw-semibold);
			color: var(--ink-mute);
		}
	}

	.avatar-btn {
		position: absolute;
		right: -2px;
		bottom: -2px;
		width: 28px;
		height: 28px;
		border-radius: var(--radius-full);
		background: var(--raised);
		border: 1px solid var(--edge-strong);
		display: grid;
		place-items: center;
		color: var(--ink-mute);
		cursor: pointer;
		transition: color var(--dur-fast) var(--ease);

		&:hover:not(:disabled) {
			color: var(--ink);
		}
	}

	.identity-text {
		display: flex;
		flex-direction: column;
		gap: 0.4375rem;
		min-width: 0;
	}

	.identity-name {
		font-size: 1.25rem;
		font-weight: var(--fw-semibold);
		letter-spacing: var(--tracking-tight);
	}

	.chips {
		display: flex;
		align-items: center;
		flex-wrap: wrap;
		gap: 0.375rem;
	}

	.chip {
		display: inline-flex;
		align-items: center;
		height: 20px;
		padding-inline: 0.4375rem;
		border-radius: var(--radius-sm);
		background: var(--tint-softer);
		font-size: var(--fs-xs);
		font-weight: var(--fw-medium);
		color: var(--ink-mute);
	}

	/* ---- fields ---- */
	.fields {
		display: flex;
		flex-direction: column;
		gap: 1.25rem;
	}

	.f-field {
		display: flex;
		flex-direction: column;
		gap: 0.375rem;

		label {
			font-size: var(--fs-xs);
			color: var(--ink-mute);
		}

		input,
		textarea {
			height: 36px;
			padding: 0 0.625rem;
			border-radius: var(--radius-sm);
			background: var(--surface);
			border: 1px solid var(--edge);
			color: var(--ink);
			font-family: var(--font-sans);
			font-size: 0.875rem;
			outline: none;
			transition:
				border-color var(--dur-fast) var(--ease),
				box-shadow var(--dur-fast) var(--ease);

			&::placeholder {
				color: var(--ink-faint);
			}
			&:focus {
				border-color: var(--accent);
				box-shadow: 0 0 0 3px var(--focus-ring);
			}
			&:disabled {
				background: var(--tint-soft);
				color: var(--ink-faint);
			}
			&.mono {
				font-family: var(--font-mono);
			}
		}

		textarea {
			height: auto;
			min-height: 72px;
			padding: 0.5rem 0.625rem;
			resize: vertical;
			line-height: var(--lh-normal);
		}
	}

	.pair {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 0.75rem;
	}

	.inline {
		display: flex;
		gap: 0.375rem;

		input {
			flex: 1;
			min-width: 0;
		}
	}

	.note {
		font-size: var(--fs-xs);
		color: var(--ink-faint);

		&.warn {
			color: var(--warn);
		}
		&.block {
			font-size: var(--fs-sm);
			color: var(--ink-mute);
			line-height: var(--lh-normal);
			margin: 0;
		}
	}

	/* ---- otp ---- */
	.otp-stage {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}

	.otp-line {
		font-size: var(--fs-sm);
		color: var(--ink-mute);

		strong {
			color: var(--ink);
			font-weight: var(--fw-semibold);
		}
	}

	.otp-boxes {
		display: flex;
		gap: 0.375rem;

		input {
			width: 40px;
			height: 44px;
			text-align: center;
			border-radius: var(--radius-sm);
			background: var(--surface);
			border: 1px solid var(--edge);
			color: var(--ink);
			font-family: var(--font-mono);
			font-size: 1.125rem;
			outline: none;
			transition:
				border-color var(--dur-fast) var(--ease),
				box-shadow var(--dur-fast) var(--ease);

			&.filled {
				border-color: var(--edge-strong);
			}
			&:focus {
				border-color: var(--accent);
				box-shadow: 0 0 0 3px var(--focus-ring);
			}
		}
	}

	.otp-actions {
		display: flex;
		align-items: center;
		gap: 0.75rem;
	}

	/* ---- api key ---- */
	.key-row {
		display: flex;
		align-items: center;
		gap: 0.375rem;

		input {
			flex: 1;
			min-width: 0;
		}
	}

	.ghost.icon {
		width: 36px;
		padding: 0;
		display: grid;
		place-items: center;
		flex: 0 0 auto;
	}

	code {
		font-family: var(--font-mono);
		font-size: 0.9em;
		color: var(--ink);
	}

	.danger-box {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		padding: 0.875rem 1rem;
		border: 1px solid var(--edge);
		border-radius: var(--radius-md);
		background: var(--danger-soft);
	}

	.danger-title {
		font-size: var(--fs-sm);
		font-weight: var(--fw-semibold);
		color: var(--ink);
	}

	.danger-line {
		font-size: var(--fs-xs);
		color: var(--ink-mute);
		line-height: var(--lh-normal);
	}

	.danger-actions {
		display: flex;
		align-items: center;
		justify-content: flex-end;
		gap: var(--space-3);
	}

	.danger-btn {
		height: 32px;
		padding-inline: 0.875rem;
		border: 0;
		border-radius: var(--radius-md);
		background: var(--danger);
		color: #fff;
		font: inherit;
		font-size: var(--fs-sm);
		font-weight: var(--fw-medium);
		cursor: pointer;

		&:hover:not(:disabled) {
			filter: brightness(1.08);
		}
		&:disabled {
			opacity: 0.6;
			cursor: not-allowed;
		}
	}

	/* ---- buttons ---- */
	.primary {
		height: 36px;
		padding-inline: 1rem;
		border: 0;
		border-radius: var(--radius-md);
		background: var(--accent);
		color: #fff;
		font: inherit;
		font-size: var(--fs-sm);
		font-weight: var(--fw-medium);
		cursor: pointer;
		transition: background var(--dur-fast) var(--ease);

		&.sm {
			height: 34px;
			padding-inline: 0.875rem;
		}
		&:hover:not(:disabled) {
			background: var(--accent-hover);
		}
		&:disabled {
			opacity: 0.55;
			cursor: not-allowed;
		}
	}

	.ghost {
		height: 36px;
		padding-inline: 0.875rem;
		border-radius: var(--radius-sm);
		border: 1px solid var(--edge);
		background: none;
		color: var(--ink);
		font: inherit;
		font-size: var(--fs-sm);
		font-weight: var(--fw-medium);
		cursor: pointer;
		transition: background var(--dur-fast) var(--ease);

		&:hover:not(:disabled) {
			background: var(--tint-soft);
		}
		&:disabled {
			opacity: 0.55;
			cursor: not-allowed;
		}
	}

	.link-btn {
		border: 0;
		background: none;
		font: inherit;
		font-size: var(--fs-sm);
		color: var(--ink-mute);
		cursor: pointer;
		padding: 0;
		transition: color var(--dur-fast) var(--ease);

		&.faint {
			color: var(--ink-faint);
		}
		&:hover:not(:disabled) {
			color: var(--ink);
		}
		&:disabled {
			opacity: 0.55;
			cursor: not-allowed;
		}
	}

	@media (max-width: 900px) {
		.section,
		.row-section {
			grid-template-columns: 1fr;
			row-gap: 1rem;
		}
		.section > :not(.section-label) {
			grid-column: 1;
		}
		.section-label {
			padding-top: 0;
		}
	}

	@media (max-width: 620px) {
		.pair {
			grid-template-columns: 1fr;
		}
		.identity {
			gap: 1rem;
		}
	}
</style>
