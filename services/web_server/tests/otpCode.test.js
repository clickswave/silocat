// The password reset code box. Run with `npm test`.
//
// Reset codes became eight letters and digits so that guessing them needs no
// per-account lockout a stranger could set off. The box used to drop anything
// but digits and stop at six, which would make every new code impossible to
// enter.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { cleanCode } from '../src/lib/otpCode.js';

test('a reset code is kept whole, however it was typed or pasted', () => {
	assert.equal(cleanCode('K7QF9M2X', 8, true), 'K7QF9M2X');
	assert.equal(cleanCode(' k7qf-9m2x ', 8, true), 'K7QF9M2X');
	assert.equal(cleanCode('K7QF 9M2X extra', 8, true), 'K7QF9M2X');
});

test('a verification code box still keeps six digits only', () => {
	assert.equal(cleanCode('12a34-567', 6), '123456');
	assert.equal(cleanCode(undefined, 6), '');
});

test('the sign-in page asks for an eight character reset code', () => {
	const page = readFileSync(new URL('../src/routes/auth/signin/+page.svelte', import.meta.url), 'utf8');
	const box = page.match(/<OtpInput[^>]*bind:value=\{resetOtp\}[^>]*>/);
	assert.ok(box, 'no reset code box on the sign-in page');
	assert.match(box[0], /length=\{8\}/);
	assert.match(box[0], /\bletters\b/);
});
