/**
 * A secondary control may be drawn smaller than 48, never tapped smaller
 * (ADR 0044, 10 October 2026). The small buttons that replaced underlined
 * words are drawn at 40 and the shelf arrows at 32, and each grows an
 * invisible round itself to 48, as the cooking screen's chips have since #88.
 *
 * Measured in Chrome, where a finger lands, and not read off the stylesheet's
 * numbers: the round sits inside the control's border, which a sum of the
 * numbers left out, so every outlined one was tapped at 46 while this test
 * said 48 (#634).
 */

import type { Browser, Page } from 'playwright';
import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import { CHROME_HOOK_MS, drawn, startChrome } from '../testing/chrome';
import { SIZES } from '../../tools/sizes.mjs';

/** Each control as a screen writes it, far enough from the page's edge for its round. */
const CONTROLS = {
	'quiet-button': '<button class="quiet-button">Rename</button>',
	'shelf-arrow': '<button class="shelf-arrow" aria-label="Later">›</button>',
	// `tap-out` sizes nothing itself: a borderless control at 32 square.
	'tap-out': '<button class="tap-out" style="width: 32px; height: 32px">×</button>',
	// And outlined, as the Writing screen's tools and the cooking chips are.
	'tap-out with a border':
		'<button class="tap-out flex h-8 w-8 shrink-0 items-center justify-center rounded-sm border">×</button>',
};

/**
 * Just short of half of 48: a point this far either side of the centre is
 * under the finger on a control tapped at 48, and misses one tapped at 47.
 */
const REACH = 23.9;

let browser: Browser;
beforeAll(async () => {
	browser = await startChrome();
}, CHROME_HOOK_MS);
afterAll(async () => {
	await browser?.close();
}, CHROME_HOOK_MS);

/** Whether a tap at each point lands on the control. */
async function lands(page: Page, points: [number, number][]): Promise<boolean[]> {
	return page.evaluate((points) => {
		const control = document.querySelector('button')!;
		return points.map(([x, y]) => {
			const hit = document.elementFromPoint(x, y);
			return hit !== null && control.contains(hit);
		});
	}, points);
}

describe('a control drawn under 48', () => {
	it.each(Object.entries(CONTROLS))(
		'%s is tapped at 48 whatever it is drawn at',
		async (_name, markup) => {
			const page = await drawn(browser, `<div style="padding: 64px">${markup}</div>`, {
				size: SIZES.phone,
			});
			try {
				const box = (await page.locator('button').boundingBox())!;
				expect(Math.min(box.width, box.height)).toBeLessThan(48);
				const x = box.x + box.width / 2;
				const y = box.y + box.height / 2;
				const across = Math.max(box.width / 2, REACH);
				expect(
					await lands(page, [
						[x, y - REACH],
						[x, y + REACH],
						[x - across, y],
						[x + across, y],
					]),
				).toEqual([true, true, true, true]);
			} finally {
				await page.close();
			}
		},
	);
});
