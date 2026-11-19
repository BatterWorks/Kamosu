/**
 * The count every screen test ends on (#632): broken here on purpose, so a
 * count that stopped counting would fail.
 */

import { afterEach, describe, expect, it } from 'vitest';
import { MOST_MAIN_BUTTONS, tooManyMainButtons } from './main-buttons';

afterEach(() => {
	document.body.innerHTML = '';
});

const buttons = (count: number) =>
	Array.from({ length: count }, (_, i) => `<button class="main-button">Act ${i + 1}</button>`).join(
		'',
	);

describe('main buttons on one screen', () => {
	it(`lets ${MOST_MAIN_BUTTONS} through`, () => {
		document.body.innerHTML = buttons(MOST_MAIN_BUTTONS);
		expect(tooManyMainButtons()).toBeUndefined();
	});

	it('names each one when there are more', () => {
		document.body.innerHTML = buttons(MOST_MAIN_BUTTONS + 1);
		expect(tooManyMainButtons()?.message).toMatch(
			/^3 main buttons on one screen, "Act 1", "Act 2", "Act 3"/,
		);
	});

	it('counts an open sheet apart from the page behind it', () => {
		document.body.innerHTML = `${buttons(2)}<div role="dialog" aria-modal="true">${buttons(1)}</div>`;
		expect(tooManyMainButtons()).toBeUndefined();
		document.body.innerHTML = `${buttons(1)}<div role="dialog" aria-modal="true">${buttons(3)}</div>`;
		expect(tooManyMainButtons()).toBeInstanceOf(Error);
	});

	it('does not count the tokens page’s specimens', () => {
		document.body.innerHTML = `<section data-specimens>${buttons(4)}</section>`;
		expect(tooManyMainButtons()).toBeUndefined();
	});

	it('does not count a secondary button', () => {
		document.body.innerHTML = `${buttons(2)}<button class="second-button">Not now</button>`;
		expect(tooManyMainButtons()).toBeUndefined();
	});
});
