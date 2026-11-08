/**
 * `just screens <folder> <path>…` (#216): a full-page picture of each path on
 * the dev instance at every size in `sizes.mjs`, saved as
 * `<folder>/<path>.<size>.png`.
 *
 * A page taller than `PICTURE_LIMIT` is photographed in pieces, top to
 * bottom with no gap and no overlap, as `<path>.<size>-1.png`, `-2.png` and
 * on (#522). WebKit refuses one picture past that limit; Chrome takes it, but
 * the limit holds in both so a page gives the same pictures in either engine.
 * A size that cannot be photographed is a fault, and the other sizes are
 * still taken.
 *
 * It fails when a page logged a console error or scrolls sideways, after
 * photographing everything, so one run reports every fault there is.
 *
 * `--webkit` among the arguments runs it in Safari's engine, which found a
 * real fault on #198 that Chrome did not show.
 *
 * Two runs over an unchanged screen write the same bytes, which is what lets
 * `just screens-same` answer "is the phone unchanged?" with no image library:
 * movement is switched off and the caret is hidden, each screen is taken only
 * once it has stopped changing, the page is not told of the resize Chrome
 * takes a full-page picture with nor of code the dev server changed, Chrome
 * paints whole tiles, and what the tools themselves do is kept off the
 * screens (`openSettled`, `holdWindowStill`, `holdCodeStill`, `openBrowser`
 * and `leaveNoTrace` in `dev.mjs`, #529). A screen that never stops changing
 * is a fault.
 */

import fs from 'node:fs';
import path from 'node:path';
import {
	ROOT,
	ToolError,
	atEverySize,
	fromRoot,
	openBrowser,
	openSettled,
	run,
	signedInArgs,
	slugOf,
} from './dev.mjs';

/**
 * The tallest or widest picture WebKit takes, in the screen's own pixels:
 * past it, `page.screenshot` throws (#522).
 */
export const PICTURE_LIMIT = 32767;

/**
 * The file a path's picture goes in at one size, or one piece of it when the
 * page was too tall for one picture.
 * @param {string} route
 * @param {string} size
 * @param {number} [piece] counted from 1
 */
export function pictureName(route, size, piece) {
	return `${slugOf(route)}.${size}${piece ? `-${piece}` : ''}.png`;
}

/**
 * The stretches of a page, top to bottom, each short enough for one picture:
 * one stretch when the page fits.
 * @param {number} height the page's height
 * @param {number} limit the tallest one picture may be, in the same unit
 * @returns {{ y: number, height: number }[]}
 */
export function piecesOf(height, limit) {
	const pieces = [];
	for (let y = 0; y < height; y += limit) pieces.push({ y, height: Math.min(limit, height - y) });
	return pieces;
}

/**
 * Remove the pictures an earlier run left for a path at one size, whole or in
 * pieces, so a folder never mixes two runs.
 * @param {string} out
 * @param {string} route
 * @param {string} size
 */
function clearPictures(out, route, size) {
	const whole = pictureName(route, size);
	// What every piece's name starts with, read off the first piece's.
	const stem = pictureName(route, size, 1).slice(0, -'1.png'.length);
	for (const name of fs.readdirSync(out)) {
		const piece = name.startsWith(stem) && /^\d+\.png$/.test(name.slice(stem.length));
		if (name === whole || piece) fs.rmSync(path.join(out, name));
	}
}

/**
 * The page's size as Playwright's own full-page picture measures it, and how
 * many screen pixels one CSS pixel is.
 * @param {import('playwright').Page} page
 */
function measure(page) {
	return page.evaluate(() => {
		const { body, documentElement: root } = document;
		return {
			width: Math.max(
				body.scrollWidth,
				root.scrollWidth,
				body.offsetWidth,
				root.offsetWidth,
				body.clientWidth,
				root.clientWidth,
			),
			height: Math.max(
				body.scrollHeight,
				root.scrollHeight,
				body.offsetHeight,
				root.offsetHeight,
				body.clientHeight,
				root.clientHeight,
			),
			ratio: window.devicePixelRatio,
		};
	});
}

/**
 * Photograph the page as it stands, whole or in pieces, and name each file
 * written.
 * @param {import('playwright').Page} page
 * @param {string} out
 * @param {string} route
 * @param {string} size
 * @returns {Promise<{ files: string[], height: number }>}
 */
async function photograph(page, out, route, size) {
	const { width, height, ratio } = await measure(page);
	// A clip is given in CSS pixels, the limit in the screen's.
	const limit = Math.floor(PICTURE_LIMIT / ratio);
	if (width > limit) throw new Error(`is ${width}px wide, wider than one picture can be`);
	const pieces = piecesOf(height, limit);
	// A page that fits is taken unclipped, as before #522, so a "before" set
	// taken by the earlier tool still compares byte for byte.
	if (pieces.length === 1) {
		const file = path.join(out, pictureName(route, size));
		await page.screenshot({ path: file, fullPage: true, animations: 'disabled' });
		return { files: [file], height };
	}
	const files = [];
	for (const [index, piece] of pieces.entries()) {
		const file = path.join(out, pictureName(route, size, index + 1));
		await page.screenshot({
			path: file,
			fullPage: true,
			clip: { x: 0, y: piece.y, width, height: piece.height },
			animations: 'disabled',
		});
		files.push(file);
	}
	// Each picture settles the page again first, and a clip does not follow a
	// page that grew: the stretch below the last piece would go unseen.
	const after = (await measure(page)).height;
	if (after > height) throw new Error(`grew from ${height}px to ${after}px while photographed`);
	return { files, height };
}

async function main() {
	const args = process.argv.slice(2);
	const webkit = args.includes('--webkit');
	const {
		name: folder,
		routes,
		origin,
		secret,
	} = await signedInArgs(
		args.filter((arg) => arg !== '--webkit'),
		'just screens <folder> <path>… [--webkit]',
	);

	const out = fromRoot(folder);
	fs.mkdirSync(out, { recursive: true });
	const browser = await openBrowser({ webkit });
	/** @type {string[]} */
	const faults = [];
	try {
		await atEverySize(browser, origin, secret, async (size, page) => {
			// What a page logs is put down to the address it is showing by then.
			const where = () => `${new URL(page.url()).pathname} at ${size}`;
			page.on('console', (message) => {
				if (message.type() === 'error') faults.push(`${where()}: console error: ${message.text()}`);
			});
			page.on('pageerror', (error) => faults.push(`${where()}: ${error}`));
			for (const route of routes) {
				try {
					// Before anything can fail, so a failed run leaves no earlier
					// run's pictures to be compared as if they were this one's.
					clearPictures(out, route, size);
					// A screen still changing would differ from one run to the next,
					// so it is a fault, though it is still photographed.
					const unsettled = await openSettled(page, origin + route);
					if (unsettled) faults.push(`${route} at ${size}: ${unsettled}`);
					const spill = await page.evaluate(
						() => document.documentElement.scrollWidth - document.documentElement.clientWidth,
					);
					if (spill > 0) faults.push(`${route} at ${size}: scrolls sideways by ${spill}px`);
					const { files, height } = await photograph(page, out, route, size);
					if (files.length > 1) {
						console.log(
							`${route} at ${size} is ${height}px tall, too tall for one picture: ${files.length} pieces`,
						);
					}
					for (const file of files) console.log(path.relative(ROOT, file));
				} catch (error) {
					faults.push(`${route} at ${size}: not photographed: ${String(error).split('\n')[0]}`);
				}
			}
		});
	} finally {
		await browser.close();
	}
	if (faults.length) {
		throw new ToolError(
			`${faults.length} fault(s) on the screens photographed:\n${faults.join('\n')}`,
		);
	}
}

if (process.argv[1] === import.meta.filename) await run(main);
