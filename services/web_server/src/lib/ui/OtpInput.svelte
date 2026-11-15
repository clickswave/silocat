<script>
	import { cleanCode } from '$lib/otpCode.js';

	let {
		value = $bindable(''),
		length = 6,
		// Letters and digits, as in a password reset code, rather than digits
		// only. No maxlength then, since a pasted code may carry a hyphen or
		// spaces that cleanCode drops.
		letters = false,
		disabled = false,
		oncomplete = undefined,
		...rest
	} = $props();

	function oninput(e) {
		value = cleanCode(e.target.value, length, letters);
		e.target.value = value;
		if (value.length === length) oncomplete?.(value);
	}
</script>

<input
	class="otp"
	type="text"
	inputmode={letters ? 'text' : 'numeric'}
	autocomplete="one-time-code"
	autocapitalize={letters ? 'characters' : 'off'}
	spellcheck="false"
	placeholder={(letters ? 'X' : '0').repeat(length)}
	maxlength={letters ? undefined : length}
	{disabled}
	{value}
	{oninput}
	{...rest}
/>

<style lang="scss">
	.otp {
		width: 100%;
		background: var(--bg);
		border: 1px solid var(--edge);
		border-radius: var(--radius-sm);
		color: var(--ink);
		font-family: var(--font-mono);
		font-size: var(--fs-h3);
		letter-spacing: 0.6em;
		text-align: center;
		text-indent: 0.6em; /* balance the trailing letter-spacing */
		padding: 0.7rem 0.5rem;
		outline: none;
		transition:
			border-color var(--dur) var(--ease),
			box-shadow var(--dur) var(--ease);

		&::placeholder {
			color: var(--ink-faint);
			opacity: 0.5;
		}
		&:focus {
			border-color: var(--accent);
			box-shadow: 0 0 0 3px var(--focus-ring);
		}
		&:disabled {
			opacity: 0.5;
		}
	}
</style>
