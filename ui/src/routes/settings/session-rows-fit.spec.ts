/**
 * Each device signed in fits its list on Settings › Signing in, in every
 * Language (#689).
 *
 * A row held the device's name on the left and its two buttons on the
 * right, on one line whatever the room. In Spanish, on an iPad mini held
 * upright, where the page stands beside the Settings menu, "Cerrar sesión"
 * and "Cambiar nombre" ran the row 23px past the window. Where the name and
 * the buttons do not fit side by side, the buttons now go under the name,
 * and wrap there if they must, as offline, where the one in your hand says
 * "Signing out waits for the server".
 *
 * jsdom lays nothing out, so the screen is drawn as the other tests draw it
 * and measured in Chrome (`src/testing/chrome.ts`), inside columns of every
 * width in `COLUMNS`, at the ordinary text size and the largest (#228).
 */

import type { Browser } from 'playwright';
import { afterAll, afterEach, beforeAll, describe, expect, it } from 'vitest';
import { cleanup, screen } from '@testing-library/svelte';
import { getLocale, locales, setLocale, type Locale } from '#lib/paraglide/runtime.js';
import { renderScreen } from '../../testing/render';
import { CHROME_HOOK_MS, drawn, startChrome } from '../../testing/chrome';
import { NARROWEST_PHONE, SIZES } from '../../../tools/sizes.mjs';
import SigningIn from './SigningIn.svelte';

/**
 * The widths of the column the screen is drawn in, in CSS pixels, every
 * 10px from 80px narrower than the narrowest phone to the width of an iPad
 * mini held upright, where #689 was found beside the Settings menu.
 */
const COLUMNS = Array.from(
	{ length: (SIZES['mini-upright'][0] - (NARROWEST_PHONE[0] - 80)) / 10 + 1 },
	(_, i) => NARROWEST_PHONE[0] - 80 + i * 10,
);
/** The root at the ordinary text size and at the largest, 16 × 1.35. */
const TEXT_SIZES = ['16px', '21.6px'];

/** An instance where nobody has added a passkey yet. */
const NO_PASSKEYS = {
	public_address: 'https://kamosu.example',
	available: true,
	unavailable_because: null,
	origin: 'https://kamosu.example',
	passkeys: [],
};

let browser: Browser;
beforeAll(async () => {
	browser = await startChrome();
}, CHROME_HOOK_MS);
afterAll(async () => {
	await browser?.close();
}, CHROME_HOOK_MS);

const session = (id: string, name: string, current = false) => ({
	id,
	name,
	created_at: '2026-09-12T09:00:00Z',
	last_used_at: '2026-09-20T18:28:00Z',
	revoked: false,
	current,
});

afterEach(() => {
	Reflect.deleteProperty(navigator, 'onLine');
});

/** The screen's markup in `language`, online or not, once its devices are listed. */
async function markupIn(language: Locale, offline: boolean): Promise<string> {
	const before = getLocale();
	setLocale(language, { reload: false });
	if (offline) Object.defineProperty(navigator, 'onLine', { value: false, configurable: true });
	try {
		renderScreen(SigningIn, {
			list_sessions: {
				sessions: [
					session('s_ipad', 'Safari · iPad', true),
					session('s_laptop', 'Firefox · Linux'),
					session('s_old', 'this browser'),
				],
			},
			list_passkeys: NO_PASSKEYS,
			list_access_keys: { access_keys: [] },
		});
		await screen.findByText('Firefox · Linux');
		return document.body.innerHTML;
	} finally {
		cleanup();
		setLocale(before, { reload: false });
	}
}

/**
 * How far each device's row reaches past the list it stands in, keyed by
 * the text size, the column's width and the row's words, wherever it does.
 * Every part of the row is measured, since a row that is too wide for its
 * list can widen its grid's column rather than scroll inside itself.
 */
async function overflows(markup: string, language: Locale) {
	const page = await drawn(browser, `<div id="column">${markup}</div>`, {
		size: [1000, 1200],
		lang: language,
	});
	try {
		return await page.evaluate(
			([columns, textSizes]) => {
				const found: Record<string, number> = {};
				const column = document.getElementById('column')!;
				// The devices are the screen's last section.
				const list = [...document.querySelectorAll('section')].at(-1)!.querySelector('ul')!;
				for (const textSize of textSizes) {
					document.documentElement.style.fontSize = textSize;
					for (const width of columns) {
						column.style.width = `${width}px`;
						const edge = list.getBoundingClientRect().right;
						for (const row of list.querySelectorAll<HTMLElement>(':scope > li')) {
							const reach = Math.max(
								...[row, ...row.querySelectorAll('*')].map(
									(part) => part.getBoundingClientRect().right,
								),
							);
							if (reach - edge > 0.5) {
								found[`${textSize}, ${width}px: ${row.textContent?.replace(/\s+/g, ' ').trim()}`] =
									reach - edge;
							}
						}
					}
				}
				return found;
			},
			[COLUMNS, TEXT_SIZES] as const,
		);
	} finally {
		await page.close();
	}
}

describe('the devices signed in', () => {
	for (const language of locales) {
		for (const offline of [false, true]) {
			it(`fit their list in ${language}${offline ? ', offline' : ''}`, async () => {
				expect(await overflows(await markupIn(language, offline), language)).toEqual({});
			});
		}
	}
});
