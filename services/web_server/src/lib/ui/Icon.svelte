<script>
	import { ICONS, resolveIcon } from './icons.js';

	let {
		/** Design-set icon name. Either this or `icon` must be given. */
		name = undefined,
		icon = undefined, // legacy alias prop, so `<Icon icon="ri:x" />` keeps working
		size = 16,
		width = undefined, // legacy alias for size
		stroke = null, // override the per-icon default weight
		filled = false, // solid variant (starred, active indicators)
		class: klass = '',
		label = null, // when set the icon is meaningful and gets a11y text
		style = ''
	} = $props();

	let key = $derived(resolveIcon(name ?? icon));
	let def = $derived(ICONS[key] || ICONS.file);
	let px = $derived(Number(width ?? size));
	let weight = $derived(stroke ?? def.w ?? 1.6);
	let solid = $derived(filled || def.fill === true);
</script>

{#if def.raw}
	<!-- eslint-disable-next-line svelte/no-at-html-tags -- static in-repo icon markup -->
	<svg
		viewBox="0 0 24 24"
		width={px}
		height={px}
		class={klass}
		style="--sc-icon-size:{px}px;{style}"
		role={label ? 'img' : 'presentation'}
		aria-label={label}
		aria-hidden={label ? null : 'true'}>{@html def.raw}</svg
	>
{:else}
	<svg
		viewBox="0 0 24 24"
		width={px}
		height={px}
		fill={solid ? 'currentColor' : 'none'}
		stroke={solid ? 'none' : 'currentColor'}
		stroke-width={solid ? null : weight}
		stroke-linecap={solid ? null : 'round'}
		stroke-linejoin={solid ? null : 'round'}
		class="{klass}{def.spin ? ' sc-spin' : ''}"
		style="--sc-icon-size:{px}px;{style}"
		role={label ? 'img' : 'presentation'}
		aria-label={label}
		aria-hidden={label ? null : 'true'}
	>
		{#each def.s as shape, i (i)}
			{#if shape.t === 'circle'}
				<circle cx={shape.cx} cy={shape.cy} r={shape.r} />
			{:else if shape.t === 'rect'}
				<rect x={shape.x} y={shape.y} width={shape.width} height={shape.height} rx={shape.rx} />
			{:else}
				<path d={shape.d} />
			{/if}
		{/each}
	</svg>
{/if}

<style>
	svg {
		display: block;
		/* Belt and braces: the width/height attributes alone let a flex or grid
		   parent squash the glyph (a 16px icon was rendering 5px wide inside a
		   modal header chip). These lock the box in every container. */
		flex: 0 0 auto;
		min-width: var(--sc-icon-size);
		min-height: var(--sc-icon-size);
	}

	/* Spinners are the only thing in this UI allowed to rotate. */
	:global(.sc-spin) {
		animation: sc-icon-spin 0.7s linear infinite;
	}

	@keyframes sc-icon-spin {
		to {
			transform: rotate(360deg);
		}
	}

	@media (prefers-reduced-motion: reduce) {
		:global(.sc-spin) {
			animation-duration: 2s;
		}
	}
</style>
