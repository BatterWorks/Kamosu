/**
 * Type follows the reader's own text size, up to 135% (#228, ADR 0046).
 *
 * The rule is written once in `ui/src/app.css`: every type size is a count of
 * `--type-unit`, and the unit follows the root's font size, which the system's
 * text-size setting moves. jsdom lays nothing out, so these read the source.
 */
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';

const read = (file: string) => readFileSync(resolve(process.cwd(), file), 'utf8');
const stylesheet = read('src/app.css');
/** The stylesheet without its comments, which may say `rem` and `px` freely. */
const rules = stylesheet.replace(/\/\*[\s\S]*?\*\//g, '');

describe('type follows the reader’s text size', () => {
	it('writes every type size as a count of the unit', () => {
		// A size's line height and tracking are declared beside it under the
		// same name with a second `--` in it (`--text-step--line-height`), and
		// are no sizes.
		const sizes = [...rules.matchAll(/(--text-[a-z-]+):\s*([^;]+);/g)].filter(
			([, name]) => !name.includes('--', 2),
		);
		expect(sizes.length).toBeGreaterThan(20);
		for (const [, name, value] of sizes) {
			// A count of the unit, or another size by name, as `cook-far` writes
			// the cooking screen's on the wide layout.
			expect(value, name).toMatch(/^(calc\([\d.]+ \* var\(--type-unit\)\)|var\(--text-[a-z-]+\))$/);
		}
	});

	it('writes no type size anywhere else in the stylesheet', () => {
		const written = [...rules.matchAll(/font-size:\s*([^;]+);/g)].map(([, value]) => value);
		for (const value of written) {
			expect(value).toMatch(/var\(--t(ext|ype)-/);
		}
	});

	it('never goes under today’s sizes and stops at 135%', () => {
		// Under 100% a field's text would fall below 16px, where iOS zooms the
		// page on focus (`app.html`). Over 135% the screens stop holding, which
		// the 200% and 310% prototypes on #228 showed.
		const units = [...rules.matchAll(/--type-unit:\s*([^;]+);/g)].map(([, value]) => value);
		expect(units).toEqual([
			'clamp(1px, calc(1rem / 16), 1.35px)',
			'clamp(1px, calc(1rem / 17), 1.35px)',
		]);
	});

	it('moves nothing but type when the root’s size moves', () => {
		// The root's font size is the system's to set from here on, so a width
		// written in rem would grow with the reader's text size, and on an
		// iPhone, whose root is 17px at rest, would differ from today at rest.
		const lengths = rules.replace(/--type-unit:[^;]+;/g, '').match(/[\d.]+rem\b/g);
		expect(lengths).toBeNull();
	});

	it('asks Chrome on Android for the system’s size', () => {
		expect(read('src/app.html')).toContain('<meta name="text-scale" content="scale" />');
	});
});
