/**
 * Every place's word fits its tab and the rail at the largest text size
 * (#600).
 *
 * The words grow with the reader's text size, and the bar and the rail do
 * not. At the type unit's 1.35px top, "COOKED" ran off a 360px phone and
 * "SHOPPING" and "SETTINGS" out of the 88px rail. Closing the words up
 * then set two side by side in the bar touching, "SHOPPINGCOOKED", so the
 * space between two words is held here too. The bar and the rail are drawn
 * here as the tests draw them, in each Language, and measured in Chrome
 * (`src/testing/chrome.ts`) with the root at 21.6px, 16 × 1.35.
 */

import type { Browser } from 'playwright';
import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import { cleanup, render } from '@testing-library/svelte';
import { getLocale, locales, setLocale, type Locale } from '#lib/paraglide/runtime.js';
import TabBar from './TabBar.svelte';
import Sidebar from './Sidebar.svelte';
import { CHROME_HOOK_MS, drawn, startChrome } from '../../testing/chrome';
import { NARROWEST_PHONE, SIZES } from '../../../tools/sizes.mjs';

const LARGEST = '21.6px';
/**
 * The least space between two words side by side, in CSS pixels, the same
 * in every Language: before #605, two pairs of Dutch words touched on a
 * 360px phone.
 */
const APART = 4;
/** The narrowest phone the bar is held to, and the phone every tool draws. */
const PHONES = { 'a 360px phone': NARROWEST_PHONE, 'a 390px phone': SIZES.phone };

let browser: Browser;
beforeAll(async () => {
	browser = await startChrome();
}, CHROME_HOOK_MS);
afterAll(async () => {
	await browser?.close();
}, CHROME_HOOK_MS);

/** The navigation's markup in `language`, the interface left in the one it was in. */
function markupIn(language: Locale, navigation: typeof TabBar | typeof Sidebar): string {
	const before = getLocale();
	setLocale(language, { reload: false });
	try {
		render(navigation, { props: { pathname: '/' } });
		return document.body.innerHTML;
	} finally {
		cleanup();
		setLocale(before, { reload: false });
	}
}

/**
 * What is wrong with the navigation's words, in CSS pixels: each place whose
 * word runs past either side of its link inside the link's border, where the
 * current place's line is drawn, or past the screen, with how far; and each
 * two words side by side less than `APART` apart, with how far apart. A word
 * is measured itself rather than by its link's scroll width, which counts
 * only the right of a centred word's overflow, in whole pixels.
 */
async function faults(markup: string, language: Locale, size: readonly number[]) {
	const page = await drawn(browser, markup, { size, lang: language, rootFontSize: LARGEST });
	try {
		return await page.evaluate((apart) => {
			const found: Record<string, number> = {};
			const words: { name: string; box: DOMRect }[] = [];
			for (const link of document.querySelectorAll<HTMLElement>('.place-link')) {
				const name = link.textContent?.trim() ?? '';
				// The word is the link's last child, after its icon.
				const range = document.createRange();
				range.selectNodeContents(link.lastChild ?? link);
				const box = range.getBoundingClientRect();
				const style = getComputedStyle(link);
				const edges = link.getBoundingClientRect();
				const left = Math.max(edges.left + parseFloat(style.borderLeftWidth), 0);
				const right = Math.min(
					edges.right - parseFloat(style.borderRightWidth),
					document.documentElement.clientWidth,
				);
				const past = Math.max(left - box.left, box.right - right);
				if (past > 0) found[`${name} cut`] = past;
				words.push({ name, box });
			}
			for (const [i, { name, box }] of words.slice(1).entries()) {
				const before = words[i];
				const sideBySide = box.top < before.box.bottom && before.box.top < box.bottom;
				const gap = box.left - before.box.right;
				if (sideBySide && gap < apart) found[`${before.name} and ${name} touch`] = gap;
			}
			return found;
		}, APART);
	} finally {
		await page.close();
	}
}

describe('the navigation at the largest text size', () => {
	for (const language of locales) {
		for (const [phone, size] of Object.entries(PHONES)) {
			it(`fits every word of the tab bar in ${language} on ${phone}`, async () => {
				expect(await faults(markupIn(language, TabBar), language, size)).toEqual({});
			});
		}
		it(`fits every word of the rail in ${language}`, async () => {
			expect(await faults(markupIn(language, Sidebar), language, SIZES['mini-sideways'])).toEqual(
				{},
			);
		});
	}
});
