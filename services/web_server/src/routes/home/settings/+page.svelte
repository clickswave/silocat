<script>
	import { onMount } from 'svelte';
	import Icon from '@iconify/svelte';
	import { countries, getCountryName } from '$lib/countries';
	import { enhance } from '$app/forms';
	import CountrySelect from '$lib/components/CountrySelect.svelte';
	import { browser } from '$app/environment';

	let { data } = $props();

	const defaultUser = {
		username: 'User',
		email: 'user@example.com',
		country: ''
	};

	let user = $derived(data.user || defaultUser);

	let theme = $state('dark');
	let emailNotifications = $state(true);
	let pushNotifications = $state(false);
	let saving = $state(false);

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

	function handlePasswordChange() {
		alert('Password reset link sent to your email.');
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
					{#if user.avatar_url}
						<img src={user.avatar_url} alt="avatar" />
					{:else}
						<div class="avatar-placeholder">
							<Icon icon="ri:user-smile-line" width="40" />
						</div>
					{/if}
					<button class="edit-avatar" title="Change Avatar">
						<Icon icon="ri:camera-line" />
					</button>
				</div>
				<div class="user-meta">
					<h3>{user.username}</h3>
					<span class="role">Pro Member</span>
					<span class="country-badge" title="Country">
						{getCountryName(user.country)}
					</span>
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
						readonly
						disabled
					/>
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

				<div class="form-group">
					<label for="email">Email Address</label>
					<input
						type="email"
						id="email"
						bind:value={profileForm.email}
						disabled
						title="Email cannot be changed"
					/>
					<span class="note">Contact support to change email.</span>
				</div>

				<div class="form-actions">
					<button type="button" class="btn secondary" onclick={handlePasswordChange}
						>Change Password</button
					>
					<button type="submit" class="btn primary" disabled={saving}>
						{saving ? 'Saving...' : 'Save Changes'}
					</button>
				</div>
			</form>
		</section>
	</div>
</div>

<style lang="scss">
	.settings-page {
		max-width: 1000px;
		margin: 0 auto;
		padding-bottom: 4rem;

		header {
			margin-bottom: 2rem;
			h1 {
				font-size: 2rem;
				margin-bottom: 0.5rem;
				color: var(--text-primary);
			}
			.subtitle {
				color: var(--text-muted);
			}
		}
	}

	.settings-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(400px, 1fr));
		gap: 2rem;
	}

	.card {
		background: var(--bg-card);
		border: 1px solid var(--border-default);
		border-radius: var(--radius-lg);
		padding: 24px;
		display: flex;
		flex-direction: column;
		gap: 20px;

		.card-header {
			display: flex;
			align-items: center;
			gap: 12px;
			padding-bottom: 16px;
			border-bottom: 1px solid var(--border-default);

			.header-icon {
				width: 40px;
				height: 40px;
				border-radius: 10px;
				background: rgba(255, 70, 85, 0.1);
				color: var(--primary);
				display: flex;
				align-items: center;
				justify-content: center;
			}
			h2 {
				font-size: 1.1rem;
				font-weight: 600;
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
		padding: 8px 0;

		.label {
			display: flex;
			flex-direction: column;
			gap: 4px;
			.title {
				font-weight: 500;
				color: var(--text-primary);
			}
			.desc {
				font-size: 0.85rem;
				color: var(--text-muted);
			}
		}

		.controls {
			display: flex;
			gap: 8px;
			background: var(--bg-input);
			padding: 4px;
			border-radius: 8px;

			.toggle-btn {
				background: transparent;
				border: none;
				color: var(--text-secondary);
				padding: 6px 12px;
				border-radius: 6px;
				cursor: pointer;
				display: flex;
				align-items: center;
				gap: 6px;
				font-size: 0.9rem;
				transition: all 0.2s;

				&.active {
					background: var(--bg-card);
					color: var(--text-primary);
					box-shadow: 0 2px 4px rgba(0, 0, 0, 0.2);
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
			transition: 0.4s;
			border: 1px solid var(--border-default);

			&.round {
				border-radius: 24px;
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
				background-color: white;
				transition: 0.4s;
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
		gap: 20px;
		margin-bottom: 10px;

		.avatar-wrapper {
			position: relative;
			width: 80px;
			height: 80px;

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
				color: white;
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
				margin: 0 0 4px 0;
				font-size: 1.25rem;
			}
			.role {
				background: rgba(255, 70, 85, 0.1);
				color: var(--primary);
				padding: 2px 8px;
				border-radius: 4px;
				font-size: 0.75rem;
				font-weight: 600;
				text-transform: uppercase;
			}
		}
	}

	.profile-form {
		display: flex;
		flex-direction: column;
		gap: 16px;

		.form-group {
			display: flex;
			flex-direction: column;
			gap: 8px;

			label {
				font-size: 0.9rem;
				font-weight: 500;
				color: var(--text-muted);
			}

			input,
			textarea {
				background: var(--bg-input);
				border: 1px solid var(--border-default);
				border-radius: 8px;
				padding: 10px 14px;
				color: var(--text-primary);
				font-family: inherit;
				font-size: 0.95rem;
				transition: border-color 0.2s;

				&:focus {
					outline: none;
					border-color: var(--primary);
				}

				&:disabled {
					opacity: 0.6;
					cursor: not-allowed;
				}
			}

			.note {
				font-size: 0.75rem;
				color: var(--text-muted);
				font-style: italic;
			}
		}

		.form-actions {
			display: flex;
			justify-content: flex-end;
			gap: 12px;
			margin-top: 8px;
		}
	}

	.btn {
		padding: 10px 20px;
		border-radius: 8px;
		font-weight: 500;
		cursor: pointer;
		border: none;
		font-size: 0.9rem;

		&.primary {
			background: var(--primary);
			color: white;
			&:hover {
				filter: brightness(1.1);
			}
		}
		&.secondary {
			background: transparent;
			border: 1px solid var(--border-default);
			color: var(--text-primary);
			&:hover {
				background: var(--nav-hover);
			}
		}
	}
</style>
