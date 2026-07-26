<script>
	import { page } from '$app/stores';
	import Icon from '$lib/ui/Icon.svelte';
	import Modal from '$lib/ui/Modal.svelte';
	import Button from '$lib/ui/Button.svelte';
	import { theme, toggleTheme } from '$lib/theme.js';
	import { shadowKey, regenerateShadowKey, clearShadowKey } from '$lib/stores/shadow.js';
	import { toast } from '$lib/toast.js';

	const links = [
		{ href: '/', label: 'Send' },
		{ href: '/pricing', label: 'Pricing' },
		{ href: '/privacy', label: 'Security' },
		{ href: '/api', label: 'API' },
		{ href: '/about', label: 'About' }
	];

	let showMobileMenu = $state(false);
	let showKeyModal = $state(false);

	let user = $derived($page.data?.user);
	let activeKey = $derived(user?.api_key || $shadowKey);

	function copyKey() {
		if (!activeKey) return;
		navigator.clipboard.writeText(activeKey);
		toast.success('Key copied', 'Keep it somewhere safe.');
	}

	function regenerate() {
		regenerateShadowKey();
		toast.success('New key generated', 'Drops made with the old key can no longer be managed here.');
	}

	function deleteKey() {
		clearShadowKey();
		showKeyModal = false;
		toast.info('Key deleted', 'This browser can no longer manage its earlier drops.');
	}
</script>

<header class="nav">
	<div class="inner">
		<a href="/" class="brand">
			<img src="/silocat-logo.png" alt="" width="26" height="26" />
			<span>silocat</span>
		</a>

		<nav class="links" aria-label="Primary">
			{#each links as link (link.href)}
				<a href={link.href} class="link" class:active={$page.url.pathname === link.href}>
					{link.label}
				</a>
			{/each}
		</nav>

		<div class="actions">
			{#if activeKey && !user}
				<button
					type="button"
					class="chip key-dot"
					aria-label="Your upload key"
					title="Your upload key"
					onclick={() => (showKeyModal = true)}
				>
					<Icon name="key" size={16} />
					<span class="dot" aria-hidden="true"></span>
				</button>
			{/if}

			<button
				type="button"
				class="chip"
				aria-label={$theme === 'dark' ? 'Switch to light theme' : 'Switch to dark theme'}
				title="Toggle theme"
				onclick={toggleTheme}
			>
				<Icon name={$theme === 'dark' ? 'sun' : 'moon'} size={16} />
			</button>

			{#if user}
				<a href="/home" class="cta desktop-only">Open app</a>
			{:else}
				<a href="/auth/signin" class="signin desktop-only">Sign in</a>
				<a href="/auth/signup" class="cta desktop-only">Sign up</a>
			{/if}

			<button
				type="button"
				class="chip mobile-only"
				aria-label="Menu"
				onclick={() => (showMobileMenu = true)}
			>
				<Icon name="menu" size={18} />
			</button>
		</div>
	</div>
</header>

<Modal open={showMobileMenu} title="Menu" onclose={() => (showMobileMenu = false)}>
	<div class="mobile-links">
		{#each links as link (link.href)}
			<a href={link.href} class="mobile-link" onclick={() => (showMobileMenu = false)}>
				{link.label}
			</a>
		{/each}
		{#if activeKey && !user}
			<button
				type="button"
				class="mobile-link as-button"
				onclick={() => {
					showMobileMenu = false;
					showKeyModal = true;
				}}
			>
				Your upload key
			</button>
		{/if}
		<div class="mobile-sep"></div>
		{#if user}
			<Button block href="/home" onclick={() => (showMobileMenu = false)}>Open app</Button>
		{:else}
			<a href="/auth/signin" class="mobile-link" onclick={() => (showMobileMenu = false)}>Sign in</a>
			<Button block href="/auth/signup" onclick={() => (showMobileMenu = false)}>Sign up</Button>
		{/if}
	</div>
</Modal>

<Modal open={showKeyModal} title="Your upload key" icon="key" onclose={() => (showKeyModal = false)}>
	<div class="key-stack">
		<p class="key-desc">
			{#if user}
				You are signed in, so uploads use your account key.
			{:else}
				This key lives in this browser and proves the anonymous drops on it are yours. Save it
				somewhere safe if you want to manage them from another device.
			{/if}
		</p>
		{#if activeKey}
			<div class="key-box">
				<span class="key-value">{activeKey}</span>
				<button type="button" class="key-copy" onclick={copyKey}>Copy</button>
			</div>
		{/if}
	</div>
	{#snippet footer()}
		{#if user}
			<Button onclick={() => (showKeyModal = false)}>Close</Button>
		{:else}
			<button type="button" class="key-delete" onclick={deleteKey}>Delete key</button>
			<div class="key-right">
				<Button variant="ghost" size="sm" onclick={regenerate}>Regenerate</Button>
				<Button size="sm" onclick={() => (showKeyModal = false)}>Close</Button>
			</div>
		{/if}
	{/snippet}
</Modal>

<style lang="scss">
	.nav {
		position: sticky;
		top: 0;
		z-index: 20;
		background: var(--bg);
		border-bottom: 1px solid var(--edge);
		font-family: var(--font-sans);
	}

	.inner {
		max-width: var(--container);
		margin: 0 auto;
		padding-inline: var(--gutter);
		height: 60px;
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-6);
	}

	.brand {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		color: var(--ink);
		text-decoration: none;
		flex: 0 0 auto;

		img {
			width: 26px;
			height: 26px;
			border-radius: 7px;
			display: block;
		}
		span {
			font-size: 1rem;
			font-weight: var(--fw-semibold);
			letter-spacing: var(--tracking-tight);
		}
	}

	.links {
		display: flex;
		align-items: center;
		gap: var(--space-1);
	}

	.link {
		padding: 0.375rem 0.625rem;
		border-radius: var(--radius-sm);
		font-size: var(--fs-sm);
		font-weight: var(--fw-medium);
		color: var(--ink-mute);
		text-decoration: none;
		transition:
			background var(--dur-fast) var(--ease),
			color var(--dur-fast) var(--ease);

		&:hover {
			background: var(--nav-hover);
			color: var(--ink);
		}
		&.active {
			color: var(--accent);
		}
	}

	.actions {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		flex: 0 0 auto;
	}

	.chip {
		position: relative;
		width: 30px;
		height: 30px;
		border: 0;
		background: none;
		border-radius: var(--radius-sm);
		display: grid;
		place-items: center;
		color: var(--ink-mute);
		cursor: pointer;
		transition:
			background var(--dur-fast) var(--ease),
			color var(--dur-fast) var(--ease);

		&:hover {
			background: var(--nav-hover);
			color: var(--ink);
		}
	}

	.key-dot .dot {
		position: absolute;
		top: 4px;
		right: 4px;
		width: 5px;
		height: 5px;
		border-radius: var(--radius-full);
		background: var(--accent);
	}

	.signin {
		padding: 0.375rem 0.5rem;
		font-size: var(--fs-sm);
		font-weight: var(--fw-medium);
		color: var(--ink-mute);
		text-decoration: none;
		transition: color var(--dur-fast) var(--ease);

		&:hover {
			color: var(--ink);
		}
	}

	.cta {
		display: flex;
		align-items: center;
		height: 32px;
		padding-inline: 0.875rem;
		border-radius: var(--radius-md);
		background: var(--accent);
		color: #fff;
		font-size: var(--fs-sm);
		font-weight: var(--fw-medium);
		text-decoration: none;
		transition: background var(--dur-fast) var(--ease);

		&:hover {
			background: var(--accent-hover);
			color: #fff;
		}
	}

	.mobile-links {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
	}
	.mobile-link {
		padding: 0.65rem 0.25rem;
		font-size: var(--fs-body);
		color: var(--ink);
		text-decoration: none;
		text-align: left;
		background: none;
		border: 0;
		font-family: inherit;
		cursor: pointer;
	}
	.mobile-sep {
		height: 1px;
		background: var(--edge);
		margin-block: var(--space-2);
	}

	.key-stack {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
	}
	.key-desc {
		margin: 0;
		font-size: var(--fs-sm);
		color: var(--ink-mute);
		line-height: var(--lh-normal);
	}
	.key-box {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		padding: 0.75rem;
		border-radius: 8px;
		background: var(--surface);
		border: 1px solid var(--edge);
	}
	.key-value {
		flex: 1;
		min-width: 0;
		font-family: var(--font-mono);
		font-size: var(--fs-sm);
		color: var(--ink-mute);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.key-copy {
		flex: 0 0 auto;
		height: 28px;
		padding-inline: 0.625rem;
		border-radius: var(--radius-sm);
		border: 1px solid var(--edge);
		background: none;
		color: inherit;
		font: inherit;
		font-size: var(--fs-xs);
		font-weight: var(--fw-medium);
		cursor: pointer;
		transition: background var(--dur-fast) var(--ease);

		&:hover {
			background: var(--tint-soft);
		}
	}
	.key-delete {
		height: 34px;
		padding-inline: 0.875rem;
		border: 0;
		background: none;
		border-radius: var(--radius-md);
		font: inherit;
		font-size: var(--fs-sm);
		color: var(--danger);
		cursor: pointer;
		margin-right: auto;
		transition: background var(--dur-fast) var(--ease);

		&:hover {
			background: var(--danger-soft);
		}
	}
	.key-right {
		display: flex;
		gap: var(--space-2);
	}

	.mobile-only {
		display: none;
	}

	@media (max-width: 860px) {
		.links,
		.desktop-only {
			display: none;
		}
		.mobile-only {
			display: grid;
		}
	}
</style>
