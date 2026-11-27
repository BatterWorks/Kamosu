/**
 * `just ux-walk <folder> [task…]` (#639): record each task of the UX audit
 * (#630) as pictures, one before every step of its shortest path and one at
 * its finish line, and ready them for an evaluator to walk one at a time.
 *
 * The tasks and their paths are declared once, in `ux-tasks.mjs`. Each step
 * names what it taps the way a person sees it, by a control's words or its
 * label for a screen reader, so a path breaks loudly when the interface
 * changes under it: a step whose control is not there fails the run with the
 * task, the step and a picture of where it stopped.
 *
 * **Never on the dev instance**, since the tasks write: they share, plan, add
 * to the list and import. Each task runs on its own throwaway instance of the
 * debug binary holding a fresh copy of the Sample Library that `just
 * sample-library` built in `.dev/sample-library`, as an ordinary instance and
 * never as a Demo, which refuses sign-in, sharing, importing a link and
 * inviting. The tool signs in as the library's first cook through `kamosu
 * recover` against that copy, since the Sample Library's people have
 * passwords nobody knows, and removes the instance when the task ends, pass
 * or fail.
 *
 * Each picture is what the phone shows, at `phone-402` in English, taken once
 * the screen has stopped changing, with every panel, menu, confirmation and
 * message caught while it is open. A step whose control is below the fold on
 * arrival gets a scroll step of its own before it, with its own picture.
 *
 * What it writes into `<folder>`:
 *
 * - `<task>/01.png`, `02.png`… one picture before each step, and the last at
 *   the finish line;
 * - `path.json`, each task's recorded path: what each picture's step did,
 *   to which control, on which screen, and how that control was reached;
 * - `depth.csv`, the Depth table: for each task, its taps, what was typed,
 *   its scrolls, the screens passed through, and for each step whether its
 *   control was visible on arrival or behind a menu or a scroll;
 * - `handover/`, the hand-over command and every task's goal and pictures,
 *   to copy whole into an evaluator's empty folder.
 *
 * Naming tasks after the folder records only those.
 */

import crypto from 'node:crypto';
import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import {
	ROOT,
	ToolError,
	hideBrowserCards,
	fromRoot,
	op,
	openBrowser,
	openSettled,
	run,
	signedInContext,
} from './dev.mjs';
import { signInAt } from './dev-session.mjs';
import { AUDIT_PHONES } from './sizes.mjs';
import { instanceEnv, leaveNothingNow, startInstance, throwawayFolder } from './throwaway.mjs';
import { FIRST_COOK, TASKS } from './ux-tasks.mjs';

/** The hand-over command, copied into each evaluator's folder as `walk.mjs`. */
const HANDOVER = path.join(path.dirname(fileURLToPath(import.meta.url)), 'ux-walk-handover.mjs');

/** The one size the tasks are walked at: the narrower of the audit's two phones. */
const SIZE = 'phone-402';

/** How long a screen must be still before it is photographed. */
const QUIET_MS = 1000;

/** How long a screen is given to settle, and a control to appear. */
const LIMIT_MS = 30_000;

/** The roles a control is looked for under. */
const CONTROLS = /** @type {const} */ ([
	'button',
	'link',
	'tab',
	'menuitem',
	'radio',
	'checkbox',
	'option',
	'switch',
	'combobox',
]);

// ── the task file's shape ────────────────────────────────────────────────────

/**
 * @typedef {object} Step one tap or one typed entry
 * @property {string} [tap] the control's words, or its label for a screen reader
 * @property {string} [type] the words typed
 * @property {string} [into] the field typed into, by its label
 * @property {boolean} [partial] the words are part of the control's name, as a day is of its date
 * @property {boolean} [opens] the tap opens a menu, so the next step's control is behind one
 */

/**
 * @typedef {object} Task
 * @property {string} id a short name, the task's folder
 * @property {string} goal the goal in plain words, as an evaluator reads it
 * @property {string | { recipe: string }} start a path, or a recipe of the library by its title
 * @property {Step[]} steps the shortest path, each step one tap or one typed entry
 * @property {string} finish words the finish line's screen shows
 * @property {boolean} [occasional] one of the occasional tasks
 */

/**
 * What is wrong with a task file, one line each, or nothing. A step names
 * what it taps as a person sees it, so anything shaped like a CSS selector
 * is refused.
 * @param {unknown[]} tasks
 * @returns {string[]}
 */
export function faultsOf(tasks) {
	/** @type {string[]} */
	const faults = [];
	const seen = new Set();
	const selector = /^[#.[]|[>~]|::?[a-z-]+\(|\[[a-z-]+=|^[a-z]+\.[a-z]/i;
	for (const [index, raw] of tasks.entries()) {
		const task = /** @type {Partial<Task>} */ (raw ?? {});
		const name = task.id ?? `task ${index + 1}`;
		if (!task.id || !/^[a-z0-9-]+$/.test(task.id))
			faults.push(`${name}: its id is not a short name`);
		if (seen.has(task.id)) faults.push(`${name}: declared twice`);
		seen.add(task.id);
		if (!task.goal?.trim()) faults.push(`${name}: it has no goal`);
		const start = task.start;
		const startsRight =
			(typeof start === 'string' && start.startsWith('/')) ||
			(typeof start === 'object' && start !== null && typeof start.recipe === 'string');
		if (!startsRight) faults.push(`${name}: its start is neither a path nor a recipe`);
		if (!task.finish?.trim()) faults.push(`${name}: it has no finish line`);
		if (!Array.isArray(task.steps) || task.steps.length === 0) {
			faults.push(`${name}: it has no steps`);
			continue;
		}
		for (const [at, step] of task.steps.entries()) {
			const where = `${name}, step ${at + 1}`;
			const tap = typeof step.tap === 'string';
			const type = typeof step.type === 'string';
			if (tap === type) {
				faults.push(`${where}: a step taps one control or types into one field`);
				continue;
			}
			const named = tap ? step.tap : step.into;
			if (!named?.trim()) faults.push(`${where}: it names nothing to ${tap ? 'tap' : 'type into'}`);
			else if (selector.test(named)) faults.push(`${where}: "${named}" is a selector, not words`);
		}
	}
	return faults;
}

// ── the Depth table ──────────────────────────────────────────────────────────

/**
 * @typedef {object} Taken one picture of a walk, and the step it comes before
 * @property {string} picture its file, in the task's folder
 * @property {string} screen the path shown
 * @property {'tap' | 'type' | 'scroll' | 'finish'} does
 * @property {string} [control] what the step taps, types into or scrolls to
 * @property {string} [typed]
 * @property {'visible' | 'menu' | 'scroll'} [reached] how the step's control was reached
 */

/**
 * The Depth table, as CSV: one row per task, its taps, what it typed, its
 * scrolls, the screens it passed through, and each step's control as it was
 * reached.
 * @param {{ id: string, taken: Taken[] }[]} walks
 */
export function depthTable(walks) {
	const cell = (/** @type {string | number} */ value) => {
		const text = String(value);
		return /[",\n]/.test(text) ? `"${text.replaceAll('"', '""')}"` : text;
	};
	const rows = walks.map(({ id, taken }) => {
		const steps = taken.filter((t) => t.does !== 'finish');
		const screens = taken
			.map((t) => t.screen)
			.filter((screen, at, all) => at === 0 || all[at - 1] !== screen);
		const acts = steps
			.filter((t) => t.does !== 'scroll')
			.map((t, at) => `${at + 1} ${t.control} (${t.reached})`);
		return [
			id,
			steps.filter((t) => t.does === 'tap').length,
			steps.filter((t) => t.does === 'type').length,
			steps.filter((t) => t.does === 'scroll').length,
			screens.length,
			screens.join(' > '),
			acts.join('; '),
		];
	});
	return [
		'task,taps,typed,scrolls,screens,screens_passed,steps',
		...rows.map((row) => row.map(cell).join(',')),
	]
		.join('\n')
		.concat('\n');
}

// ── the hand-over ────────────────────────────────────────────────────────────

/**
 * Write the hand-over folder: the command as `walk.mjs`, and one pack holding
 * each task's goal and its pictures in order, which the command hands over
 * one at a time. The folder holds nothing else, so it can be copied whole
 * into an evaluator's empty folder.
 * @param {string} folder
 * @param {{ id: string, goal: string, pictures: string[] }[]} walks each picture a PNG's path
 */
export function writeHandover(folder, walks) {
	fs.rmSync(folder, { recursive: true, force: true });
	fs.mkdirSync(folder, { recursive: true });
	fs.copyFileSync(HANDOVER, path.join(folder, 'walk.mjs'));
	const tasks = walks.map(({ id, goal, pictures }) => ({
		id,
		goal,
		pictures: pictures.map((file) => fs.readFileSync(file).toString('base64')),
	}));
	fs.writeFileSync(path.join(folder, 'walk-pack.json'), `${JSON.stringify({ tasks })}\n`);
}

// ── walking one task ─────────────────────────────────────────────────────────

/** A step that could not be taken, with where it stopped. */
export class Stopped extends ToolError {}

/**
 * The control a step names, among the roles a person taps, or the field it
 * types into, by its label.
 * @param {import('playwright').Page} page
 * @param {Step} step
 */
function controlOf(page, step) {
	if (step.type !== undefined) {
		const label = /** @type {string} */ (step.into);
		return page
			.getByLabel(label, { exact: true })
			.or(page.getByRole('searchbox', { name: label, exact: true }))
			.or(page.getByRole('textbox', { name: label, exact: true }))
			.or(page.getByPlaceholder(label, { exact: true }));
	}
	const name = /** @type {string} */ (step.tap);
	const exact = !step.partial;
	let found = page.getByRole(CONTROLS[0], { name, exact });
	for (const role of CONTROLS.slice(1)) {
		found = found.or(page.getByRole(/** @type {any} */ (role), { name, exact }));
	}
	return found;
}

/**
 * The first of a locator's matches a person can see, waiting up to `limit`
 * for one, or nothing.
 * @param {import('playwright').Locator} locator
 * @param {number} limit
 */
async function firstVisible(locator, limit) {
	const deadline = Date.now() + limit;
	for (;;) {
		const count = await locator.count();
		for (let at = 0; at < count; at++) {
			const one = locator.nth(at);
			if (await one.isVisible()) return one;
		}
		if (Date.now() > deadline) return undefined;
		await new Promise((done) => setTimeout(done, 200));
	}
}

/**
 * Whether a control sits inside the phone's screen as it stands, with nothing
 * over its middle: a control under the tab bar, which stays at the foot of
 * the screen, is not in view.
 * @param {import('playwright').Locator} control
 */
async function inView(control) {
	return control.evaluate((element) => {
		const box = element.getBoundingClientRect();
		const inside =
			box.top >= 0 && box.bottom <= innerHeight && box.left >= 0 && box.right <= innerWidth;
		if (!inside) return false;
		const top = document.elementFromPoint(box.left + box.width / 2, box.top + box.height / 2);
		return top !== null && (element === top || element.contains(top));
	});
}

/**
 * Wait until the screen has stopped changing where it stands, after a tap:
 * no request open and none finished for `QUIET_MS`, nothing in the document
 * changed for as long, every font in and every picture drawn. `openSettled`
 * has watched the document for changes since the task's first screen.
 * @param {import('playwright').Page} page
 * @param {{ open: Set<unknown>, last: number }} requests
 * @returns {Promise<string | undefined>} what was still changing, or nothing once settled
 */
async function settle(page, requests) {
	const deadline = Date.now() + LIMIT_MS;
	for (;;) {
		const look = await page
			.evaluate(async () => {
				// Only the pictures on the phone's screen: one further down, lazy,
				// is not asked for until it is scrolled to.
				const shown = [...document.images].filter((img) => {
					const box = img.getBoundingClientRect();
					return box.bottom > 0 && box.top < innerHeight && box.width > 0;
				});
				await Promise.race([
					Promise.all([document.fonts.ready, ...shown.map((img) => img.decode().catch(() => {}))]),
					new Promise((done) => setTimeout(done, 5000)),
				]);
				return {
					sinceChange: performance.now() - /** @type {any} */ (window).__lastChange,
					drawn: shown.every((img) => img.complete),
				};
			})
			.catch(() => undefined);
		const reasons = !look
			? ['the page was loading']
			: [
					requests.open.size > 0 && 'a request is open',
					Date.now() - requests.last < QUIET_MS && 'a request just ended',
					look.sinceChange < QUIET_MS && 'the page just changed',
					!look.drawn && 'a picture is loading',
				].filter(Boolean);
		if (reasons.length === 0) return undefined;
		if (Date.now() > deadline)
			return `still changing after ${LIMIT_MS / 1000}s: ${reasons.join(', ')}`;
		await page.waitForTimeout(100);
	}
}

/**
 * Photograph what the phone shows, the test browser's own cards kept out.
 * @param {import('playwright').Page} page
 * @param {string} file
 */
async function picture(page, file) {
	await hideBrowserCards(page, 'en');
	await page.screenshot({ path: file, animations: 'disabled', caret: 'hide' });
}

/**
 * Walk one task's path in a page signed in to `origin`, a picture before each
 * step and one at the finish line, into `out`. A control that is not there,
 * or a finish line not reached, stops the walk with a picture of where it
 * stopped and what the screen held, for a screen reader, beside it.
 * @param {import('playwright').Page} page
 * @param {string} origin
 * @param {Task & { start: string }} task its start already a path
 * @param {string} out the task's own folder
 * @param {{ limit?: number }} [options] how long a control is waited for, a test waiting less
 * @returns {Promise<Taken[]>}
 */
export async function walkTask(page, origin, task, out, { limit = LIMIT_MS } = {}) {
	fs.rmSync(out, { recursive: true, force: true });
	fs.mkdirSync(out, { recursive: true });
	const requests = { open: new Set(), last: Date.now() };
	page.on('request', (request) => {
		// A page loading anew, as the Language screen does, leaves the old
		// one's requests unanswered and unreported, as `openSettled` knows (#635).
		if (request.isNavigationRequest() && request.frame() === page.mainFrame()) {
			requests.open.clear();
		}
		requests.open.add(request);
		requests.last = Date.now();
	});
	for (const event of /** @type {const} */ (['requestfinished', 'requestfailed'])) {
		page.on(event, (request) => {
			requests.open.delete(request);
			requests.last = Date.now();
		});
	}
	/** @type {Taken[]} */
	const taken = [];
	const screen = () => {
		const url = new URL(page.url());
		return url.pathname + url.search;
	};
	const take = async (/** @type {Omit<Taken, 'picture' | 'screen'>} */ step) => {
		const name = `${String(taken.length + 1).padStart(2, '0')}.png`;
		await picture(page, path.join(out, name));
		taken.push({ picture: name, screen: screen(), ...step });
	};
	/** @param {string} where @param {string} why */
	const stop = async (where, why) => {
		const file = path.join(out, 'stopped.png');
		await picture(page, file).catch(() => {});
		const held = await page
			.locator('body')
			.ariaSnapshot()
			.catch(() => '');
		fs.writeFileSync(path.join(out, 'stopped.aria.txt'), `${held}\n`);
		throw new Stopped(
			`${task.id}, ${where}: ${why}\n  stopped on ${screen()}, pictured in ${path.relative(ROOT, file)}`,
		);
	};

	const unsettled = await openSettled(page, origin + task.start);
	if (unsettled) await stop('its start', `the screen did not settle: ${unsettled}`);
	let behindMenu = false;
	for (const [at, step] of task.steps.entries()) {
		const where = `step ${at + 1}`;
		const named = /** @type {string} */ (step.tap ?? step.into);
		const control = await firstVisible(controlOf(page, step), limit);
		if (!control) {
			await stop(where, `there is no ${step.tap !== undefined ? 'control' : 'field'} "${named}"`);
			return taken;
		}
		/** @type {'visible' | 'menu' | 'scroll'} */
		let reached = behindMenu ? 'menu' : 'visible';
		if (!(await inView(control))) {
			await take({ does: 'scroll', control: named });
			// To the middle of the screen, clear of the bars at its top and foot.
			await control.evaluate((element) => element.scrollIntoView({ block: 'center' }));
			const still = await settle(page, requests);
			if (still) await stop(where, `the screen did not settle after scrolling: ${still}`);
			// A menu is the deeper of the two: the scroll is counted on its own.
			if (!behindMenu) reached = 'scroll';
		}
		if (step.tap !== undefined) {
			await take({ does: 'tap', control: named, reached });
			await control.click();
		} else {
			await take({ does: 'type', control: named, typed: step.type, reached });
			await control.fill(/** @type {string} */ (step.type));
		}
		behindMenu = Boolean(step.opens);
		const still = await settle(page, requests);
		if (still) await stop(where, `the screen did not settle after it: ${still}`);
	}
	const finish = await firstVisible(page.getByText(task.finish, { exact: false }), limit);
	if (!finish) await stop('its finish line', `the screen does not show "${task.finish}"`);
	await take({ does: 'finish' });
	return taken;
}

// ── the run ──────────────────────────────────────────────────────────────────

/**
 * A fresh copy of the Sample Library `just sample-library` built, made into
 * `to` as an ordinary instance's data: every file and folder but the Demo's
 * own copy and the folder's marker.
 * @param {string} from
 * @param {string} to
 */
function copySampleLibrary(from, to) {
	fs.mkdirSync(to, { recursive: true });
	for (const entry of fs.readdirSync(from, { withFileTypes: true })) {
		if (['as-a-demo', 'SAMPLE-LIBRARY'].includes(entry.name)) continue;
		fs.cpSync(path.join(from, entry.name), path.join(to, entry.name), { recursive: true });
	}
}

/**
 * Sign in to a throwaway instance as `name`, through a recovery link that
 * `kamosu recover` mints against its data, with a password made here and
 * forgotten.
 * @param {string} binary
 * @param {string} data
 * @param {string} api
 * @param {string} name
 */
async function signInAs(binary, data, api, name) {
	const link = execFileSync(binary, ['recover', name], {
		env: instanceEnv(data, 0),
		encoding: 'utf8',
	}).trim();
	return signInAt(`${api}/auth/recover`, {
		link,
		password: crypto.randomBytes(18).toString('base64url'),
		session_name: 'ux-walk',
	});
}

/**
 * A task's start as a path: a recipe named by its title is found in the
 * library, and must be found once.
 * @param {string} api
 * @param {string} secret
 * @param {Task['start']} start
 */
async function startOf(api, secret, start) {
	if (typeof start === 'string') return start;
	const { recipes } = await op(api, secret, 'search_recipes', { query: start.recipe });
	const named = recipes.filter((/** @type {{ title: string }} */ r) => r.title === start.recipe);
	if (named.length !== 1) {
		throw new ToolError(
			`the Sample Library holds ${named.length} recipes titled "${start.recipe}", not one`,
		);
	}
	return `/recipes/${named[0].branch_id}`;
}

async function main() {
	const args = process.argv.slice(2);
	const binaryAt = args.indexOf('--binary');
	const binary = binaryAt >= 0 ? args.splice(binaryAt, 2)[1] : undefined;
	const [folder, ...only] = args;
	if (!folder || !binary) throw new ToolError('usage: just ux-walk <folder> [task…]');
	const faults = faultsOf(TASKS);
	if (faults.length) throw new ToolError(`ui/tools/ux-tasks.mjs:\n${faults.join('\n')}`);
	const unknown = only.filter((id) => !TASKS.some((t) => t.id === id));
	if (unknown.length) throw new ToolError(`there is no task ${unknown.join(', ')}`);
	const tasks = only.length ? TASKS.filter((t) => only.includes(t.id)) : TASKS;

	const library = path.join(ROOT, '.dev', 'sample-library');
	if (!fs.existsSync(path.join(library, 'kamosu.db')))
		throw new ToolError(
			'there is no Sample Library in .dev/sample-library: `just sample-library` builds it',
		);
	const out = fromRoot(folder);
	fs.mkdirSync(out, { recursive: true });
	// What an earlier run wrote goes first, so a run that fails leaves no
	// table or hand-over of an older one beside its pictures.
	for (const stale of ['path.json', 'depth.csv', 'handover']) {
		fs.rmSync(path.join(out, stale), { recursive: true, force: true });
	}
	for (const task of tasks) fs.rmSync(path.join(out, task.id), { recursive: true, force: true });

	const scratch = throwawayFolder('ux-walk');
	/** @type {{ id: string, goal: string, taken: Taken[] }[]} */
	const walks = [];
	const browser = await openBrowser();
	try {
		for (const task of tasks) {
			const data = path.join(scratch, task.id);
			copySampleLibrary(library, data);
			const log = path.join(out, `${task.id}.log`);
			const instance = await startInstance(binary, data, log);
			let walked = false;
			try {
				const secret = await signInAs(binary, data, instance.api, FIRST_COOK);
				const start = await startOf(instance.api, secret, task.start);
				const context = await signedInContext(
					browser,
					instance.origin,
					secret,
					AUDIT_PHONES[SIZE],
					'en',
					{ touch: true },
				);
				try {
					const page = await context.newPage();
					/** @type {string[]} */
					const errors = [];
					page.on('pageerror', (error) => errors.push(String(error)));
					const taken = await walkTask(
						page,
						instance.origin,
						{ ...task, start },
						path.join(out, task.id),
					).catch((error) => {
						if (error instanceof Stopped && errors.length) {
							error.message += `\n  the page also failed on its own:\n${errors.join('\n')}`;
						}
						throw error;
					});
					if (errors.length) {
						throw new ToolError(`${task.id}: the page failed on its own:\n${errors.join('\n')}`);
					}
					walks.push({ id: task.id, goal: task.goal, taken });
					walked = true;
					console.log(`${task.id}: ${taken.length} pictures`);
				} finally {
					await context.close();
				}
			} finally {
				await instance.stop();
				fs.rmSync(data, { recursive: true, force: true });
				// A failed task's log is kept beside its pictures, to be read.
				if (walked) fs.rmSync(log, { force: true });
			}
		}
	} finally {
		await browser.close();
		leaveNothingNow();
	}
	fs.writeFileSync(path.join(out, 'path.json'), `${JSON.stringify(walks, null, '\t')}\n`);
	fs.writeFileSync(path.join(out, 'depth.csv'), depthTable(walks));
	writeHandover(
		path.join(out, 'handover'),
		walks.map(({ id, goal, taken }) => ({
			id,
			goal,
			pictures: taken.map((t) => path.join(out, id, t.picture)),
		})),
	);
	console.log(
		`wrote ${path.relative(ROOT, out)}: ${walks.length} task(s), the Depth table and handover/`,
	);
}

if (process.argv[1] === import.meta.filename) await run(main);
