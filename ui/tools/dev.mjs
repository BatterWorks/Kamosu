/**
 * What every dev tool shares (#216): where the dev instance is, how a Session
 * kept on disk reaches a browser, and which browser that is.
 *
 * The dev instance's address and its account come from the gitignored `.env`
 * at the repo root. Nothing here prints the password or a Session's Secret:
 * a tool is handed the path of a cookie file and gives the cookie to its
 * browser itself.
 */

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { SIZES } from './sizes.mjs';

/** The repo root. Every path a recipe is given is read from here. */
export const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');

/** Where the Sessions these tools signed in keep their cookies. */
export const SESSIONS_DIR = path.join(ROOT, '.dev', 'sessions');

/** Where prototypes live. Gitignored: a prototype is thrown away once chosen from. */
export const PROTOTYPES_DIR = path.join(ROOT, '.dev', 'prototypes');

/** Something the person running the recipe can put right. Printed without a stack. */
export class ToolError extends Error {}

/**
 * Run a tool's `main`, and end the process on what it found.
 * @param {() => Promise<void>} main
 */
export async function run(main) {
	try {
		await main();
	} catch (error) {
		if (!(error instanceof ToolError)) throw error;
		console.error(`error: ${error.message}`);
		process.exitCode = 1;
	}
}

/**
 * The `KEY=value` lines of an env file. Quotes around a value are dropped.
 * @param {string} text
 * @returns {Record<string, string>}
 */
export function readEnv(text) {
	/** @type {Record<string, string>} */
	const found = {};
	for (const line of text.split('\n')) {
		const match = /^\s*([A-Za-z_][A-Za-z0-9_]*)\s*=\s*(.*?)\s*$/.exec(line);
		if (!match) continue;
		const [, key, value] = match;
		found[key] = value.replace(/^(["'])(.*)\1$/, '$2');
	}
	return found;
}

/** The repo-root `.env`, read afresh. */
export function devEnv() {
	const file = path.join(ROOT, '.env');
	if (!fs.existsSync(file)) {
		throw new ToolError('there is no .env at the repo root to name the dev instance');
	}
	return readEnv(fs.readFileSync(file, 'utf8'));
}

/**
 * One value `.env` has to hold.
 * @param {Record<string, string>} env
 * @param {string} key
 */
export function need(env, key) {
	if (!env[key]) throw new ToolError(`.env does not set ${key}`);
	return env[key];
}

/**
 * The dev instance as a browser reaches it: over HTTPS, since a Session
 * cookie is Secure and a browser sends it nowhere else.
 */
export function devOrigin(env = devEnv()) {
	return `https://${need(env, 'KAMOSU_DEV_HOST')}`;
}

/**
 * The cookie file of the Session a label names.
 * @param {string} label
 */
export function sessionFile(label) {
	if (!/^[A-Za-z0-9][A-Za-z0-9._-]*$/.test(label ?? '')) {
		throw new ToolError(
			'a Session label is letters, digits, dots, dashes and underscores, since it names a file',
		);
	}
	return path.join(SESSIONS_DIR, `${label}.cookie`);
}

/**
 * The Secret a cookie file holds. The path is read from the repo root, which
 * is how `just dev-session` prints it.
 * @param {string} file
 */
export function readSecret(file) {
	file = file && path.resolve(ROOT, file);
	if (!file || !fs.existsSync(file)) {
		throw new ToolError(`there is no cookie file at ${file}; run \`just dev-session <label>\``);
	}
	return fs.readFileSync(file, 'utf8').trim();
}

/**
 * Call one Operation at the web Door as the Session a Secret belongs to.
 * @param {string} origin
 * @param {string} secret
 * @param {string} name
 * @param {unknown} [input]
 * @returns {Promise<any>}
 */
export async function op(origin, secret, name, input = {}) {
	const response = await reach(`${origin}/api/op/${name}`, {
		method: 'POST',
		headers: { 'content-type': 'application/json', cookie: `__Host-kamosu_session=${secret}` },
		body: JSON.stringify(input),
	});
	const answer = await response.json().catch(() => null);
	if (response.status === 401) throw new SignedOut(`${name} was refused: this Session has ended`);
	if (!answer?.ok) {
		throw new ToolError(`${name} was refused: ${answer?.error?.message ?? response.status}`);
	}
	return answer.result;
}

/** The dev instance said the Session asking has ended, and nothing else. */
export class SignedOut extends ToolError {}

/**
 * `fetch`, saying what to do when the dev instance is not there to answer.
 * @param {string} url
 * @param {RequestInit} init
 */
export async function reach(url, init) {
	try {
		return await fetch(url, init);
	} catch (error) {
		const why = error instanceof Error && error.cause ? `: ${error.cause}` : '';
		throw new ToolError(
			`the dev instance did not answer at ${new URL(url).origin}${why}\nIs it running? \`just dev-start\` starts it.`,
		);
	}
}

/**
 * The Session a Secret belongs to, or nothing when the dev instance says it
 * has ended. Any other trouble is thrown: an instance that is down or failing
 * has not said the Session is over, and a cookie must not be dropped on it.
 * @param {string} origin
 * @param {string} secret
 * @returns {Promise<{ id: string, name: string } | undefined>}
 */
export async function sessionOf(origin, secret) {
	try {
		const { sessions } = await op(origin, secret, 'list_sessions');
		const own = sessions.find((/** @type {{ current: boolean }} */ s) => s.current);
		if (!own) throw new ToolError('the dev instance did not say which Session this cookie is');
		return own;
	} catch (error) {
		if (error instanceof SignedOut) return undefined;
		throw error;
	}
}

/**
 * What `screens` and `proto-new` are both given: a cookie file after
 * `--session`, a name, and the paths to open. Checked before a browser starts,
 * the Session included: a dead one would capture the sign-in screen at every
 * size and call it done.
 * @param {string[]} args
 * @param {string} usage
 */
export async function signedInArgs(args, usage) {
	const rest = [...args];
	const at = rest.indexOf('--session');
	const session = at >= 0 ? rest.splice(at, 2)[1] : undefined;
	const [name, ...routes] = rest;
	if (!name || routes.length === 0 || !session) throw new ToolError(`usage: ${usage}`);
	const strays = routes.filter((route) => !route.startsWith('/'));
	if (strays.length) {
		throw new ToolError(`a path starts with a slash, as in /recipes: ${strays.join(' ')}`);
	}
	const origin = devOrigin();
	const secret = readSecret(session);
	if (!(await sessionOf(origin, secret))) {
		throw new ToolError(`the Session in ${session} has ended; sign in again`);
	}
	return { name, routes, origin, secret };
}

/**
 * Open each size in turn as a signed-in window with one page in it.
 * @param {import('playwright').Browser} browser
 * @param {string} origin
 * @param {string} secret
 * @param {(size: string, page: import('playwright').Page, context: import('playwright').BrowserContext) => Promise<void>} visit
 */
export async function atEverySize(browser, origin, secret, visit) {
	for (const [size, dimensions] of Object.entries(SIZES)) {
		const context = await signedInContext(browser, origin, secret, dimensions);
		try {
			await visit(size, await context.newPage(), context);
		} finally {
			await context.close();
		}
	}
}

/** The names Chrome and Chromium go by on a path, most likely first. */
const CHROME_NAMES = ['google-chrome', 'google-chrome-stable', 'chromium', 'chromium-browser'];

/**
 * The Chrome these tools drive: the one `.env` names in `KAMOSU_DEV_BROWSER`,
 * else the first on the path.
 * @param {Record<string, string>} env
 * @param {string} [searchPath]
 */
export function chromePath(env, searchPath = process.env.PATH ?? '') {
	if (env.KAMOSU_DEV_BROWSER) return env.KAMOSU_DEV_BROWSER;
	for (const dir of searchPath.split(path.delimiter).filter(Boolean)) {
		for (const name of CHROME_NAMES) {
			const candidate = path.join(dir, name);
			if (fs.existsSync(candidate)) return candidate;
		}
	}
	throw new ToolError(
		'no Chrome or Chromium is on the path; name one in .env as KAMOSU_DEV_BROWSER',
	);
}

/**
 * Start the browser. `webkit` is Safari's engine, which Playwright downloads
 * itself; anything else is the Chrome already on this machine.
 * @param {{ webkit?: boolean }} [options]
 */
export async function openBrowser({ webkit = false } = {}) {
	const playwright = await import('playwright');
	if (!webkit) {
		return playwright.chromium.launch({
			executablePath: chromePath(devEnv()),
			// Chrome paints part of a tile again where it can, and the edges it
			// smooths came out a shade apart from one run to the next: 117 pixels
			// at the corners of buttons on Settings (#529). Painting whole tiles
			// gave the same picture 8 times in 8 where three versions came before.
			args: ['--disable-partial-raster'],
		});
	}
	try {
		return await playwright.webkit.launch();
	} catch (error) {
		throw new ToolError(
			`Safari's engine did not start. Fetch it once with \`cd ui && npx playwright install webkit\`.\n${String(error).split('\n')[0]}`,
		);
	}
}

/**
 * A browser context of one size, signed in to the dev instance in English.
 *
 * The service worker is kept out: it would answer from its own cache, and
 * these tools exist to show what the code on disk draws today.
 * @param {import('playwright').Browser} browser
 * @param {string} origin
 * @param {string} secret
 * @param {number[]} size
 */
export async function signedInContext(browser, origin, secret, [width, height]) {
	const context = await browser.newContext({
		viewport: { width, height },
		serviceWorkers: 'block',
		reducedMotion: 'reduce',
	});
	await context.addInitScript(() => localStorage.setItem('PARAGLIDE_LOCALE', 'en'));
	await holdWindowStill(context);
	await holdCodeStill(context);
	await leaveNoTrace(context);
	await context.addCookies([
		{
			// `__Host-` takes a cookie for this host alone, with no Domain, so it
			// is given by its address rather than by a domain and a path (#368).
			name: '__Host-kamosu_session',
			value: secret,
			url: `${new URL(origin).origin}/`,
			httpOnly: true,
			secure: true,
			sameSite: 'Lax',
		},
	]);
	return context;
}

/**
 * Chrome takes a full-page picture by shrinking the window to 1×1 and back
 * (#529). Told of that, the app switched to the phone layout and back, built
 * every screen again and read its data again while the picture was being
 * taken, so a picture caught a pane empty or at "No tags yet", or the
 * Settings menu scrolled. The tools' window is never resized by anyone, so a
 * resize to any other size than it opened at is the picture's, and the page
 * is not told.
 * @param {import('playwright').BrowserContext} context
 */
export async function holdWindowStill(context) {
	await context.addInitScript(() => {
		const opened = [innerWidth, innerHeight];
		addEventListener(
			'resize',
			(event) => {
				if (innerWidth !== opened[0] || innerHeight !== opened[1]) {
					event.stopImmediatePropagation();
				}
			},
			true,
		);
	});
}

/**
 * The dev server tells every page it serves when code changes, over a
 * WebSocket: an "update" swaps the changed modules in place, a "full-reload"
 * loads the page again, an "error" covers it with the dev server's own
 * notice until the next update. `just check` and `just test` write the
 * translations again, which sent the first two to a run that followed them,
 * and the pictures it took then were of a page half swapped or loading
 * again, waiting on module requests Chrome never sent (#529). Each path is
 * opened fresh, so the tools' window already shows the code on disk, and
 * code that does not build fails there on its own: it is told of none of
 * the three, and hears the rest of what the server says.
 * @param {import('playwright').BrowserContext} context
 */
export async function holdCodeStill(context) {
	/** @param {string | Buffer} message */
	const messageType = (message) => {
		try {
			return JSON.parse(String(message))?.type;
		} catch {
			return undefined;
		}
	};
	await context.routeWebSocket(
		() => true,
		(socket) => {
			const server = socket.connectToServer();
			socket.onMessage((message) => server.send(message));
			server.onMessage((message) => {
				if (!['update', 'full-reload', 'error'].includes(messageType(message)))
					socket.send(message);
			});
		},
	);
}

/**
 * Keep what the tools themselves do off the screens they photograph (#529),
 * so two runs over unchanged code draw the same thing.
 *
 * - **The tools' own Session's "Last used".** Every request a run makes moves
 *   it to the minute, and Signing in shows it. It is shown as when that
 *   Session began, which holds while `just dev-session tools` keeps reusing
 *   it. Every other Session is shown as the instance says.
 * - **The recipes a run opens.** Opening a recipe tells the instance, which
 *   moves it up Home's "recently opened" shelf, so photographing a recipe
 *   reorders Home for the next run. The tools' browser does not tell it:
 *   the shelf follows only what a person opened. A person opening a recipe
 *   between two runs is a real change, and the pictures say so.
 *
 * The routes see every request because `signedInContext` keeps the service
 * worker out: a worker's own requests would pass them by.
 * @param {import('playwright').BrowserContext} context
 */
export async function leaveNoTrace(context) {
	await context.route('**/api/op/list_sessions', async (route) => {
		const response = await route.fetch();
		const answer = await response.json().catch(() => undefined);
		if (!answer?.ok) return route.fulfill({ response });
		for (const session of answer.result.sessions) {
			if (session.current) session.last_used_at = session.created_at;
		}
		await route.fulfill({ response, json: answer });
	});
	await context.route('**/api/op/note_recipe_opened', (route) =>
		route.fulfill({
			json: { ok: true, result: { lineage_id: '', opened_at: new Date(0).toISOString() } },
		}),
	);
}

/**
 * How long a screen must go with no request open and nothing in it changing
 * before it counts as settled (#529). Long enough to see across the gap
 * between one answer arriving and the request it sets off.
 */
const QUIET_MS = 1000;

/** How long a screen is given to settle before it is photographed anyway. */
const SETTLE_LIMIT_MS = 30_000;

/** How long one look at the page waits for its fonts and pictures. */
const PASS_LIMIT_MS = 5000;

/**
 * Whether Playwright said the page loaded again while it waited on it.
 * @param {unknown} error
 */
function navigated(error) {
	return /interrupted by another navigation|Execution context was destroyed/.test(String(error));
}

/**
 * Go to a path, again if the page loads something else on its own first.
 * @param {import('playwright').Page} page
 * @param {string} url
 * @param {number} deadline
 * @returns {Promise<boolean>} whether it arrived before the deadline
 */
async function arrive(page, url, deadline) {
	for (;;) {
		try {
			await page.goto(url);
			return true;
		} catch (error) {
			if (!navigated(error)) throw error;
			if (Date.now() > deadline) return false;
		}
	}
}

/** The pages already watching their own document for changes. */
const watching = new WeakSet();

/**
 * Open a path and wait until the screen has stopped changing, or say why it
 * did not within `SETTLE_LIMIT_MS`.
 *
 * Settled is: no request open and none finished for `QUIET_MS`, nothing in
 * the document changed for as long, every font in, and every picture asked
 * for and drawn (#529). Playwright's own "network idle" is not enough: it is
 * reached once, early, and a request the app starts later, such as a list
 * read only once the instance has said who is signed in, is not waited for.
 * Two runs then caught a pane at "Loading…", a card that waits on the
 * offline library and on the Person, or a heading in the fallback font,
 * depending on which was quicker that time.
 * @param {import('playwright').Page} page
 * @param {string} url
 * @param {{ limit?: number }} [options] how long to wait, in ms; a test waits less
 * @returns {Promise<string | undefined>} what was still changing, or nothing once settled
 */
export async function openSettled(page, url, { limit = SETTLE_LIMIT_MS } = {}) {
	if (!watching.has(page)) {
		watching.add(page);
		// Before the page's own scripts, so the first change they make counts.
		await page.addInitScript(() => {
			const mark = () => {
				/** @type {any} */ (window).__lastChange = performance.now();
			};
			mark();
			new MutationObserver(mark).observe(document, {
				subtree: true,
				childList: true,
				attributes: true,
				characterData: true,
			});
		});
	}
	const open = new Set();
	let lastRequest = Date.now();
	/** @param {import('playwright').Request} request */
	const began = (request) => {
		// Playwright never says how a request the page left behind ended:
		// once the page loads anew, what the old one asked for is forgotten.
		if (request.isNavigationRequest() && request.frame() === page.mainFrame()) open.clear();
		open.add(request);
		lastRequest = Date.now();
	};
	/** @param {import('playwright').Request} request */
	const ended = (request) => {
		open.delete(request);
		lastRequest = Date.now();
	};
	page.on('request', began);
	page.on('requestfinished', ended);
	page.on('requestfailed', ended);
	try {
		const deadline = Date.now() + limit;
		const { origin } = new URL(url);
		const before = page.url();
		const stopped = `still changing after ${limit / 1000}s`;
		if (!(await arrive(page, url, deadline))) return `${stopped}: the page kept loading again`;
		for (;;) {
			const look = await page
				.evaluate(async (most) => {
					for (const img of document.querySelectorAll('img[loading=lazy]')) {
						/** @type {HTMLImageElement} */ (img).loading = 'eager';
					}
					// A font or a picture that never arrives would hold its promise
					// forever, and with it the limit this loop promises.
					await Promise.race([
						Promise.all([
							document.fonts.ready,
							...[...document.images].map((img) => img.decode().catch(() => {})),
						]),
						new Promise((done) => setTimeout(done, most)),
					]);
					const [load] = /** @type {PerformanceNavigationTiming[]} */ (
						performance.getEntriesByType('navigation')
					);
					return {
						sinceChange: performance.now() - /** @type {any} */ (window).__lastChange,
						fonts: document.fonts.status,
						drawn: [...document.images].every((img) => img.complete),
						reloaded: load?.type === 'reload',
					};
				}, PASS_LIMIT_MS)
				.catch((error) => {
					if (navigated(error)) return undefined;
					throw error;
				});
			// A page can still load again without the tools asking: a screen's
			// own reload, or the dev server started again, which its client
			// answers with one (#529; the dev server's own reloads are kept out
			// by `holdCodeStill`). A reload is waited out like any other change.
			// One that brought back the path shown before this one is sent on
			// to this one again. A screen that sends itself to that
			// path keeps its own page rather than loading it again, and is left
			// there, as /settings/account sends itself to Signing in.
			const sentBack =
				look?.reloaded && before !== 'about:blank' && before !== url && page.url() === before;
			if (look === undefined || sentBack) {
				if (sentBack && !(await arrive(page, url, deadline))) {
					return `${stopped}: the page kept loading again`;
				}
				if (Date.now() > deadline) return `${stopped}: the page loaded again`;
				lastRequest = Date.now();
				continue;
			}
			const sinceRequest = Date.now() - lastRequest;
			const reasons = [
				open.size > 0 &&
					`waiting on ${[...new Set([...open].map((request) => request.url().slice(origin.length)))].join(', ')}`,
				open.size === 0 && sinceRequest < QUIET_MS && 'a request just ended',
				look.sinceChange < QUIET_MS && 'the page just changed',
				look.fonts !== 'loaded' && 'a font is loading',
				!look.drawn && 'a picture is loading',
			].filter(Boolean);
			if (reasons.length === 0) return undefined;
			if (Date.now() > deadline) return `${stopped}: ${reasons.join(', ')}`;
			await page.waitForTimeout(100);
		}
	} finally {
		page.off('request', began);
		page.off('requestfinished', ended);
		page.off('requestfailed', ended);
	}
}

/**
 * A path as part of a file name: `/recipes/b_12?tab=notes` is `recipes-b_12-tab-notes`.
 * @param {string} route
 */
export function slugOf(route) {
	const slug = route.replace(/[^A-Za-z0-9_]+/g, '-').replace(/^-+|-+$/g, '');
	return slug || 'home';
}

/**
 * Where a recipe's folder argument points, read from the repo root.
 * @param {string} folder
 */
export function fromRoot(folder) {
	return path.resolve(ROOT, folder);
}
