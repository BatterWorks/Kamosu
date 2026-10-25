import { describe, expect, it } from 'vitest';
import { phoneName, setupCode, whichPhone } from './sharing';

describe('the setup code', () => {
	it("is the format's name, the address and the key, split at one space each", () => {
		const code = setupCode('https://recipes.kamosu.example/', 'ak_secret');
		expect(code).toBe('kamosu1 https://recipes.kamosu.example ak_secret');
		expect(code.split(' ')).toEqual(['kamosu1', 'https://recipes.kamosu.example', 'ak_secret']);
	});
});

describe('which phone this is', () => {
	it('reads an iPhone, an Android phone and anything else', () => {
		expect(whichPhone('Mozilla/5.0 (iPhone)', true)).toBe('apple');
		expect(whichPhone('Mozilla/5.0 (Linux; Android 14; Pixel 8)', false)).toBe('android');
		expect(whichPhone('Mozilla/5.0 (X11; Linux x86_64)', false)).toBe('other');
	});

	it('names the key for the phone', () => {
		expect(phoneName('Mozilla/5.0 (iPad; CPU OS 18_0)')).toBe('iPad');
		expect(phoneName('Mozilla/5.0 (iPhone; CPU iPhone OS 18_0)')).toBe('iPhone');
	});
});
