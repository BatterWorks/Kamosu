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
		headers: { 'content-type': 'application/json', cookie: `kamosu_session=${secret}` },
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
		await visit(size, await context.newPage(), context);
		await context.close();
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
		return playwright.chromium.launch({ executablePath: chromePath(devEnv()) });
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
	await context.addCookies([
		{
			name: 'kamosu_session',
			value: secret,
			domain: new URL(origin).hostname,
			path: '/',
			httpOnly: true,
			secure: true,
			sameSite: 'Lax',
		},
	]);
	return context;
}

/**
 * Open a path and wait until the screen has stopped changing: the network is
 * quiet, the fonts are in, and every picture further down has been asked for.
 * @param {import('playwright').Page} page
 * @param {string} url
 */
export async function openSettled(page, url) {
	await page.goto(url);
	await page.waitForLoadState('networkidle');
	await page.evaluate(async () => {
		for (const img of document.querySelectorAll('img[loading=lazy]')) {
			/** @type {HTMLImageElement} */ (img).loading = 'eager';
		}
		await document.fonts.ready;
	});
	await page.waitForLoadState('networkidle');
	await page.evaluate(() =>
		Promise.all([...document.images].map((img) => img.decode().catch(() => {}))),
	);
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
