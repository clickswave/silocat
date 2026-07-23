<script>
	import IconButton from './IconButton.svelte';
	import { theme, toggleTheme } from '$lib/theme.js';
</script>

<IconButton
	icon={$theme === 'dark' ? 'ri:sun-line' : 'ri:moon-line'}
	label={$theme === 'dark' ? 'Switch to light theme' : 'Switch to dark theme'}
	onclick={toggleTheme}
/>
