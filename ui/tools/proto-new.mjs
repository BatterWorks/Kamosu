/**
 * `just proto-new <issue>-<slug> <path>…` (#216): the start of a prototype, as
 * a standalone static copy of the screens the dev instance draws today.
 *
 * It makes `.dev/prototypes/<issue>-<slug>/` holding:
 *
 * - `captured/<path>.<size>.html`, one page for each path at each size in
 *   `sizes.mjs`. A page is the screen's `<body>` without its scripts, so
 *   nothing in it runs, saves or goes anywhere.
 * - `photos/`, every photograph those pages show.
 * - `assets/`, a copy of the stylesheet that ships, its fonts and its icons.
 * - `index.html`, a link to every page.
 *
 * Pages name what they load from the folder's root (`/assets/app.css`), so a
 * build script can write option pages anywhere in the folder and they still
 * find it. `just proto-serve` serves the folder as that root.
 *
 * Running it again with more paths adds to the folder and redraws the index.
 */

import fs from 'node:fs';
import path from 'node:path';
import {
	PROTOTYPES_DIR,
	ROOT,
	ToolError,
	atEverySize,
	openBrowser,
	openSettled,
	run,
	signedInArgs,
	slugOf,
} from './dev.mjs';
import { SIZES } from './sizes.mjs';

/**
 * The cards that belong to the browser doing the capture and not to the
 * screen: "your library isn't on this phone yet" and its kin. They are found
 * by their titles, read from the message file so a reworded card is still
 * found.
 */
function browserCardTitles() {
	const messages = JSON.parse(fs.readFileSync(path.join(ROOT, 'ui/messages/en.json'), 'utf8'));
	return Object.entries(messages)
		.filter(([key]) => /^offline_(\w+_)?title(_\w+)?$/.test(key))
		.map(([, text]) => String(text));
}

/**
 * The file a photograph is kept in: its address, made safe for a file name.
 * @param {string} src
 */
export function photoName(src) {
	return src.replace(/^\/api\/photographs\//, '').replace(/[^A-Za-z0-9._-]+/g, '_');
}

/**
 * Text as it is written inside HTML.
 * @param {string} text
 */
function escaped(text) {
	return text.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/"/g, '&quot;');
}

/**
 * One captured page, whole.
 * @param {{ title: string, htmlAttributes: string, styles: string, body: string }} capture
 */
export function pageOf({ title, htmlAttributes, styles, body }) {
	return (
		`<!doctype html>\n<html${htmlAttributes}>\n<head>\n<meta charset="utf-8">\n` +
		'<meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover">\n' +
		`<title>${escaped(title)}</title>\n` +
		// No icon is asked for, so no missing one is logged.
		'<link rel="icon" href="data:,">\n' +
		'<link rel="stylesheet" href="/assets/app.css">\n' +
		`${styles}</head>\n${body}\n</html>\n`
	);
}

/**
 * The index: every captured page, by path and size.
 * @param {string} name
 * @param {string[]} pages file names under `captured/`
 */
export function indexOf(name, pages) {
	/** @type {Map<string, string[]>} */
	const bySlug = new Map();
	for (const page of [...pages].sort()) {
		const [slug, size] = page.replace(/\.html$/, '').split('.');
		bySlug.set(slug, [...(bySlug.get(slug) ?? []), size]);
	}
	const order = Object.keys(SIZES);
	let list = '';
	for (const [slug, sizes] of bySlug) {
		const links = sizes
			.sort((a, b) => order.indexOf(a) - order.indexOf(b))
			.map((size) => {
				const dimensions = SIZES[/** @type {keyof typeof SIZES} */ (size)];
				const measure = dimensions ? ` ${dimensions[0]} × ${dimensions[1]}` : '';
				return `<li><a href="captured/${slug}.${size}.html">${size}</a>${measure}</li>`;
			})
			.join('\n');
		list += `<h2>${escaped(slug)}</h2>\n<ul>\n${links}\n</ul>\n`;
	}
	return (
		'<!doctype html>\n<html lang="en">\n<head>\n<meta charset="utf-8">\n' +
		'<meta name="viewport" content="width=device-width, initial-scale=1">\n' +
		`<title>${escaped(name)}</title>\n<link rel="icon" href="data:,">\n` +
		'<style>body{font:16px/1.5 system-ui,sans-serif;margin:24px;max-width:40rem}li{margin:4px 0}</style>\n' +
		`</head>\n<body>\n<h1>${escaped(name)}</h1>\n` +
		'<p>The screens as the dev instance drew them when this was captured. ' +
		'Open each at the size it names.</p>\n' +
		`${list}</body>\n</html>\n`
	);
}

/**
 * Read the screen a page is showing into something that stands alone. Runs in
 * the browser.
 * @param {string[]} cardTitles
 */
function capture(cardTitles) {
	// What was typed is a property; a copy of the markup only carries attributes.
	for (const input of document.querySelectorAll('input')) input.setAttribute('value', input.value);
	for (const area of document.querySelectorAll('textarea')) area.textContent = area.value;

	const body = /** @type {HTMLElement} */ (document.body.cloneNode(true));
	for (const script of body.querySelectorAll('script')) script.remove();
	for (const card of body.querySelectorAll('section[aria-label]')) {
		if (cardTitles.includes(card.getAttribute('aria-label') ?? '')) card.remove();
	}
	// Nothing behind a link was captured, so a link goes nowhere.
	for (const link of body.querySelectorAll('a[href]')) link.setAttribute('href', '#');

	/** @type {string[]} */
	const photos = [];
	for (const img of body.querySelectorAll('img')) {
		img.removeAttribute('srcset');
		img.removeAttribute('loading');
		const src = img.getAttribute('src') ?? '';
		if (src.startsWith('/api/photographs/')) {
			photos.push(src);
			// Marked here and named outside, by `photoName`: this function runs
			// in the browser, where nothing of this file but itself exists.
			img.setAttribute('src', 'kamosu-photo:' + encodeURIComponent(src));
		}
	}

	// A component's own styles. In development they sit in the head, one
	// element each; the stylesheet that ships is linked from the copy instead.
	const styles = [...document.head.querySelectorAll('style')]
		.filter((style) => !(style.getAttribute('data-vite-dev-id') ?? '').endsWith('/src/app.css'))
		.map((style) => `<style>${style.textContent}</style>\n`)
		.join('');

	const attributes = [...document.documentElement.attributes].map((a) => [a.name, a.value]);

	return { title: document.title, attributes, styles, body: body.outerHTML, photos };
}

/** A photograph's address as `capture` left it in the markup. */
const PHOTO_MARK = /kamosu-photo:([A-Za-z0-9%._~!*'()-]+)/g;

async function main() {
	const usage = 'just proto-new <issue>-<slug> <path>…   as in: just proto-new 216-tools /recipes';
	const { name, routes, origin, secret } = await signedInArgs(process.argv.slice(2), usage);
	if (!/^\d+-[a-z0-9][a-z0-9-]*$/.test(name)) throw new ToolError(`usage: ${usage}`);

	const folder = path.join(PROTOTYPES_DIR, name);
	const captured = path.join(folder, 'captured');
	const photos = path.join(folder, 'photos');
	fs.mkdirSync(captured, { recursive: true });
	fs.mkdirSync(photos, { recursive: true });
	fs.cpSync(path.join(ROOT, 'assets'), path.join(folder, 'assets'), { recursive: true });

	const cardTitles = browserCardTitles();
	const browser = await openBrowser();
	/** @type {string[]} */
	const errors = [];
	try {
		await atEverySize(browser, origin, secret, async (size, page, context) => {
			page.on('pageerror', (error) => errors.push(`${page.url()} at ${size}: ${error}`));
			for (const route of routes) {
				await openSettled(page, origin + route);
				const got = await page.evaluate(capture, cardTitles);
				for (const src of got.photos) {
					const file = path.join(photos, photoName(src));
					if (fs.existsSync(file)) continue;
					const answer = await context.request.get(origin + src);
					if (!answer.ok()) {
						errors.push(`${route} at ${size}: a photograph did not come (${answer.status()})`);
						continue;
					}
					fs.writeFileSync(file, await answer.body());
				}
				const body = got.body.replaceAll(
					PHOTO_MARK,
					(_, src) => `/photos/${photoName(decodeURIComponent(src))}`,
				);
				const htmlAttributes = got.attributes
					.map(([name, value]) => ` ${name}="${escaped(value)}"`)
					.join('');
				const file = path.join(captured, `${slugOf(route)}.${size}.html`);
				fs.writeFileSync(file, pageOf({ ...got, htmlAttributes, body }));
				console.log(path.relative(ROOT, file));
			}
		});
	} finally {
		await browser.close();
	}

	const pages = fs.readdirSync(captured).filter((file) => file.endsWith('.html'));
	fs.writeFileSync(path.join(folder, 'index.html'), indexOf(name, pages));
	console.log(`${path.relative(ROOT, folder)}: ${pages.length} pages, index.html lists them`);
	if (errors.length) {
		throw new ToolError(`the capture is not whole:\n${errors.join('\n')}`);
	}
}

if (process.argv[1] === import.meta.filename) await run(main);
