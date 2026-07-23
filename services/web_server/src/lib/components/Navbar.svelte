<script>
	import SiloCatLogo from '$lib/assets/silo-cat.png';
	import { page } from '$app/stores';
	import Icon from '@iconify/svelte';
	import { shadowKey, regenerateShadowKey, clearShadowKey } from '$lib/stores/shadow.js';
	import { Button, IconButton, Modal, Copy, ThemeToggle } from '$lib/ui';

	let showKeyModal = $state(false);
	let showMobileMenu = $state(false);

	const links = [
		{ href: '/api', label: 'API' },
		{ href: '/pricing', label: 'Pricing' },
		{ href: '/about', label: 'About' },
		{ href: '/privacy', label: 'Privacy' }
	];

	let activeKey = $derived($page.data.user ? $page.data.user.api_key : $shadowKey);
</script>

<nav class="nav">
	<div class="inner">
		<a href="/" class="brand">
			<img src={SiloCatLogo} alt="" />
			<span>silocat</span>
		</a>

		<div class="links desktop-only">
			{#each links as link (link.href)}
				<a href={link.href} class="link" class:active={$page.url.pathname === link.href}>{link.label}</a>
			{/each}
		</div>

		<div class="actions">
			{#if activeKey}
				<button
					class="key-dot desktop-only"
					onclick={() => (showKeyModal = true)}
					aria-label="Upload key"
					title="Upload key"
				>
					<Icon icon="ri:key-2-line" width={16} />
					<span class="dot" aria-hidden="true"></span>
				</button>
			{/if}
			<ThemeToggle />
			<a href="/auth/signin" class="link desktop-only">Sign in</a>
			<span class="desktop-only">
				<Button size="sm" href="/auth/signup">Sign up</Button>
			</span>
			<span class="mobile-only">
				<IconButton icon="ri:menu-line" label="Menu" onclick={() => (showMobileMenu = true)} />
			</span>
		</div>
	</div>
</nav>

<Modal open={showMobileMenu} title="Menu" onclose={() => (showMobileMenu = false)}>
	<div class="mobile-links">
		{#each links as link (link.href)}
			<a href={link.href} class="mobile-link" onclick={() => (showMobileMenu = false)}>{link.label}</a>
		{/each}
		{#if activeKey}
			<button
				class="mobile-link as-button"
				onclick={() => {
					showMobileMenu = false;
					showKeyModal = true;
				}}
			>
				Upload key
			</button>
		{/if}
		<div class="mobile-sep"></div>
		<a href="/auth/signin" class="mobile-link" onclick={() => (showMobileMenu = false)}>Sign in</a>
		<Button block href="/auth/signup" onclick={() => (showMobileMenu = false)}>Sign up</Button>
	</div>
</Modal>

<Modal open={showKeyModal} title="Upload key" icon="ri:key-2-line" onclose={() => (showKeyModal = false)}>
	<div class="key-stack">
		<p class="key-desc">
			{#if $page.data.user}
				You are signed in. Uploads use your account's API key.
			{:else}
				This browser holds a local API key so you can manage anonymous uploads across sessions, no
				account needed.
			{/if}
		</p>

		{#if activeKey}
			<div class="key-box">
				<code>{activeKey}</code>
				<Copy text={activeKey} label="Copy key" size="sm" />
			</div>
		{/if}

		{#if !$page.data.user}
			<div class="key-actions">
				<Button variant="ghost" size="sm" onclick={regenerateShadowKey}>Regenerate</Button>
				<Button
					variant="danger"
					size="sm"
					onclick={() => {
						clearShadowKey();
						showKeyModal = false;
					}}
				>
					Delete key
				</Button>
			</div>
		{/if}
	</div>
</Modal>

<style lang="scss">
	.nav {
		position: sticky;
		top: 0;
		z-index: 50;
		background: var(--bg);
		border-bottom: 1px solid var(--edge);
	}

	.inner {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-5);
		max-width: var(--container);
		margin-inline: auto;
		padding: var(--space-3) var(--gutter);
	}

	.brand {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		font-size: 1.05rem;
		font-weight: var(--fw-black);
		letter-spacing: var(--tracking-tight);
		color: var(--ink);

		img {
			width: 26px;
			height: 26px;
		}
		&:hover {
			color: var(--ink);
		}
	}

	.links {
		display: flex;
		align-items: center;
		gap: var(--space-5);
	}

	.link {
		color: var(--ink-mute);
		font-size: var(--fs-sm);
		font-weight: var(--fw-medium);
		transition: color var(--dur) var(--ease);

		&:hover,
		&.active {
			color: var(--ink);
		}
	}

	.actions {
		display: flex;
		align-items: center;
		gap: var(--space-3);
	}

	.key-dot {
		position: relative;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 32px;
		height: 32px;
		background: transparent;
		border: none;
		border-radius: var(--radius-sm);
		color: var(--ink-mute);
		cursor: pointer;
		transition:
			background var(--dur) var(--ease),
			color var(--dur) var(--ease);

		&:hover {
			background: var(--tint-soft);
			color: var(--ink);
		}

		.dot {
			position: absolute;
			top: 6px;
			right: 6px;
			width: 6px;
			height: 6px;
			background: var(--ok);
			border-radius: var(--radius-full);
		}
	}

	.mobile-only {
		display: none;
	}

	.mobile-links {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
	}

	.mobile-link {
		display: block;
		width: 100%;
		padding: 0.7rem 0.75rem;
		border-radius: var(--radius-sm);
		color: var(--ink);
		font-size: var(--fs-body);
		font-weight: var(--fw-medium);
		text-align: left;
		transition: background var(--dur-fast) var(--ease);

		&.as-button {
			background: none;
			border: none;
			font-family: inherit;
			cursor: pointer;
		}

		&:hover {
			background: var(--tint-soft);
			color: var(--ink);
		}
	}

	.mobile-sep {
		height: 1px;
		background: var(--edge);
		margin: var(--space-2) 0;
	}

	.key-stack {
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
	}

	.key-desc {
		color: var(--ink-mute);
		font-size: var(--fs-sm);
		line-height: var(--lh-normal);
		margin: 0;
	}

	.key-box {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-2);
		background: var(--bg);
		border: 1px solid var(--edge);
		border-radius: var(--radius-sm);
		padding: var(--space-2) var(--space-3);

		code {
			font-size: var(--fs-sm);
			color: var(--ink);
			overflow: hidden;
			text-overflow: ellipsis;
			white-space: nowrap;
		}
	}

	.key-actions {
		display: flex;
		gap: var(--space-2);
	}

	@media (max-width: 900px) {
		.desktop-only {
			display: none !important;
		}
		.mobile-only {
			display: inline-flex;
		}
	}
</style>
