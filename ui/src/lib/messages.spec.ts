/**
 * Every message file holds every phrase (#496).
 *
 * Paraglide compiles a phrase a file leaves out to the English one, with no
 * warning and no build error. A Dutch screen missing a phrase would show it in
 * English and nobody would notice, so this holds each Reading Language's file
 * to the English one: the same phrases, each taking the same words to fill in,
 * and a plural where the English has one.
 */
import { readdirSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { extractLocaleFromNavigator, locales } from '#lib/paraglide/runtime.js';

const folder = resolve(process.cwd(), 'messages');
const read = (locale: string): Record<string, unknown> =>
	JSON.parse(readFileSync(resolve(folder, `${locale}.json`), 'utf8'));

const english = read('en');
/** The words a phrase is handed to fill in, such as `{count}`. */
const filled = (phrase: unknown): string[] =>
	[...new Set(JSON.stringify(phrase).match(/\{\w+\}/g) ?? [])].sort();

describe('every message file', () => {
	it('is one of the Languages the interface offers, and each has one', () => {
		const files = readdirSync(folder)
			.filter((file) => file.endsWith('.json'))
			.map((file) => file.replace(/\.json$/, ''))
			.sort();
		expect(files).toEqual([...locales].sort());
	});

	it.each(locales.filter((locale) => locale !== 'en'))('%s holds every phrase', (locale) => {
		const file = read(locale);
		expect(Object.keys(file).sort()).toEqual(Object.keys(english).sort());
		for (const [key, phrase] of Object.entries(english)) {
			expect(filled(file[key]), key).toEqual(filled(phrase));
			expect(Array.isArray(file[key]), key).toBe(Array.isArray(phrase));
		}
	});
});

describe('a browser set to Dutch (#496)', () => {
	afterEach(() => vi.unstubAllGlobals());

	it('is read as Dutch, so a fresh install and the sign-in screen start in Dutch', () => {
		vi.stubGlobal('navigator', { languages: ['nl-NL', 'en-GB'] });
		expect(extractLocaleFromNavigator()).toBe('nl');
		vi.stubGlobal('navigator', { languages: ['nl-BE'] });
		expect(extractLocaleFromNavigator()).toBe('nl');
	});
});
