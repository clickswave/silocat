<script>
	import '../lib/global.scss';
	import favicon from '$lib/assets/favicon.svg';

	let { data, children } = $props();
	let { user } = data;

</script>

<svelte:head>
	<link rel="icon" href={favicon} />
</svelte:head>

{@render children()}
