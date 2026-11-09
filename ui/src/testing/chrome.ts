/**
 * The Chrome the dev tools drive, for the few screen tests that need a page
 * laid out. jsdom lays nothing out, so such a test draws its markup as the
 * other tests do and hands it here, with the stylesheet the binary embeds, to
 * be measured (#385, #600). The stylesheet is the one `just css` wrote into
 * `assets/`, so a change to `ui/src/app.css` reaches these tests only once
 * that has run.
 */

import fs from 'node:fs';
import path from 'node:path';
import { chromium, type Browser, type Page } from 'playwright';
import { chromePath, devEnv, ROOT } from '../../tools/dev.mjs';

/**
 * How long Chrome gets to start. Alone it starts in a second or two, but
 * under a full `just test`, beside the Rust build, it once missed vitest's
 * default 10 s for a hook (#554).
 */
const CHROME_START_MS = 60_000;

/**
 * How long a hook that starts or stops Chrome waits: ten seconds more than
 * Chrome's start, so a Chrome that cannot start fails with the error below,
 * not with the hook's.
 */
export const CHROME_HOOK_MS = CHROME_START_MS + 10_000;

/** Chrome, the one `.env` names, else the first on the path. */
export async function startChrome(): Promise<Browser> {
	// A machine with no dev instance to name still runs the tests.
	const env = fs.existsSync(path.join(ROOT, '.env')) ? devEnv() : {};
	const executablePath = chromePath(env);
	try {
		return await chromium.launch({ executablePath, timeout: CHROME_START_MS });
	} catch (error) {
		throw new Error(`Chrome did not start from ${executablePath}`, { cause: error });
	}
}

interface Drawing {
	/** The window's size in CSS pixels. */
	size: readonly number[];
	/** The page's language, which the stylesheet's `:lang()` rules read. */
	lang?: string;
	/** The root's font size, which is the reader's text size (#228). */
	rootFontSize?: string;
}

/**
 * A page holding `markup` and the stylesheet, its fonts loaded. The caller
 * closes it.
 */
export async function drawn(
	browser: Browser,
	markup: string,
	{ size: [width, height], lang = 'en', rootFontSize }: Drawing,
): Promise<Page> {
	const page = await browser.newPage({ viewport: { width, height } });
	const rootStyle = rootFontSize ? ` style="font-size: ${rootFontSize}"` : '';
	// The stylesheet and its fonts from where the binary serves them.
	await page.route('http://kamosu.test/**', (route) => {
		const url = new URL(route.request().url());
		if (url.pathname === '/') {
			return route.fulfill({
				contentType: 'text/html',
				body: `<!doctype html><html lang="${lang}"${rootStyle}><head><meta charset="utf-8"><link rel="stylesheet" href="/assets/app.css"></head><body>${markup}</body></html>`,
			});
		}
		const file = path.join(ROOT, url.pathname);
		return fs.existsSync(file) ? route.fulfill({ path: file }) : route.fulfill({ status: 404 });
	});
	await page.goto('http://kamosu.test/');
	await page.evaluate(() => document.fonts.ready);
	return page;
}
