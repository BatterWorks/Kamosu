/**
 * `just screens <folder> <path>…` (#216): a full-page picture of each path on
 * the dev instance at every size in `sizes.mjs`, saved as
 * `<folder>/<path>.<size>.png`.
 *
 * It fails when a page logged a console error or scrolls sideways, after
 * photographing everything, so one run reports every fault there is.
 *
 * `--webkit` among the arguments runs it in Safari's engine, which found a
 * real fault on #198 that Chrome did not show.
 *
 * Two runs over an unchanged screen write the same bytes, which is what lets
 * `just screens-same` answer "is the phone unchanged?" with no image library:
 * movement is switched off and the caret is hidden.
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
 * The file a path's picture goes in at one size.
 * @param {string} route
 * @param {string} size
 */
export function pictureName(route, size) {
	return `${slugOf(route)}.${size}.png`;
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
				await openSettled(page, origin + route);
				const spill = await page.evaluate(
					() => document.documentElement.scrollWidth - document.documentElement.clientWidth,
				);
				if (spill > 0) faults.push(`${route} at ${size}: scrolls sideways by ${spill}px`);
				const file = path.join(out, pictureName(route, size));
				await page.screenshot({ path: file, fullPage: true, animations: 'disabled' });
				console.log(path.relative(ROOT, file));
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
