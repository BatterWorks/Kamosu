/**
 * `just ux-measure <folder> [--everyday]` (#629): count what each screen
 * holds, at the UX audit's sizes and in every Language the interface speaks,
 * photograph it, and write one table.
 *
 * The measures are the research note's, section 3
 * (`docs/research/ux-reading-crowding-and-finding.md`), counted by
 * `ux-measures.mjs`. Depth is the one left out: `just ux-walk` counts it
 * from each task's recorded path (#639).
 *
 * **Never on the dev instance.** Opening the cooking screen starts a cooking
 * and the share screen makes a public Share Link, and the dev library is the
 * only copy. The tool starts two throwaway instances of the debug binary on
 * 127.0.0.1, each on a spare port with its own data folder under the system's
 * temp folder, and removes both when it ends, pass or fail:
 *
 * - **the library**: a copy of `.dev/data`, its database taken with SQLite's
 *   own backup on a read-only connection and its files copied beside it
 *   (Backups and migration snapshots left out), signed in as the dev account
 *   in `.env`;
 * - **the empty one**: a fresh instance whose first Person the tool makes, for
 *   the empty screens and the four pages a secret link opens, which are
 *   measured signed out, as the stranger holding the link sees them.
 *
 * The dev database is read before and after the run, row by row, and a change
 * is a fault. So is a page's own error. Either fails the run once everything
 * is written, as `just screens` does. Stopped with Ctrl-C, the run stops both
 * instances and removes their folder before it ends.
 *
 * What it writes into `<folder>`: `measures.csv`, one row per screen, size and
 * Language; `details.json`, the same rows with everything behind each number
 * (which sentences, which controls); and `pictures/<locale>/`, a whole-page
 * picture of each screen in English and French. `--everyday` measures the
 * everyday screens alone, to time them before the full set.
 */

import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { DatabaseSync, backup } from 'node:sqlite';
import {
	ROOT,
	ToolError,
	devEnv,
	fromRoot,
	need,
	op,
	openBrowser,
	openSettled,
	run,
	hideBrowserCards,
	signedInContext,
	toolContext,
} from './dev.mjs';
import { signInAt } from './dev-session.mjs';
import { photograph } from './screens.mjs';
import { AUDIT_PHONES, MOUSE_SIZES, NARROWEST_PHONE, SIZES } from './sizes.mjs';
import { leaveNothingNow, startInstance, throwawayFolder } from './throwaway.mjs';
import { measureInPage, valuesOf } from './ux-measures.mjs';

/** The sizes everyday screens are measured at: the two audit phones and the wide layout's two. */
const EVERYDAY_SIZES = {
	...AUDIT_PHONES,
	'ipad-upright': SIZES['ipad-upright'],
	macbook: SIZES.macbook,
};

/** Every size a row can name, the 360 pass's included. */
const ALL_SIZES = { ...EVERYDAY_SIZES, narrowest: NARROWEST_PHONE };

/** The Languages photographed: English, and French, the longest. */
const PICTURED = ['en', 'fr'];

/** A developer's page, left out of the audit. */
const NOT_MEASURED = ['/tokens'];

/** Each Language's phrases, read once. */
/** @type {Map<string, Record<string, string>>} */
const phrasebooks = new Map();

/**
 * The interface's phrases in one Language.
 * @param {string} locale
 */
function phrases(locale) {
	let found = phrasebooks.get(locale);
	if (!found) {
		found = JSON.parse(
			fs.readFileSync(path.join(ROOT, 'ui', 'messages', `${locale}.json`), 'utf8'),
		);
		phrasebooks.set(locale, /** @type {Record<string, string>} */ (found));
	}
	return /** @type {Record<string, string>} */ (found);
}

/** The Languages the interface speaks, one message file each, English first. */
function locales() {
	return fs
		.readdirSync(path.join(ROOT, 'ui', 'messages'))
		.filter((name) => name.endsWith('.json'))
		.map((name) => name.slice(0, -'.json'.length))
		.sort((a, b) => (a === 'en' ? -1 : b === 'en' ? 1 : a.localeCompare(b)));
}

/**
 * Every page whose route folder has no `[param]` in it, read from the routes
 * so a new page is measured without anyone remembering to add it.
 */
function plainRoutes() {
	const routes = path.join(ROOT, 'ui', 'src', 'routes');
	/** @type {string[]} */
	const found = [];
	/** @param {string} dir */
	const walk = (dir) => {
		for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
			if (!entry.isDirectory() || entry.name.includes('[')) continue;
			const full = path.join(dir, entry.name);
			if (fs.existsSync(path.join(full, '+page.svelte'))) {
				found.push('/' + path.relative(routes, full).split(path.sep).join('/'));
			}
			walk(full);
		}
	};
	walk(routes);
	return found.sort();
}

/**
 * The first app the import page offers, read from the interface's own list,
 * for the page that imports from another app.
 */
function firstOtherApp() {
	const list = fs.readFileSync(path.join(ROOT, 'ui', 'src', 'lib', 'other-apps.ts'), 'utf8');
	const kind = /kind: '([a-z-]+)'/.exec(list)?.[1];
	if (!kind) throw new ToolError('ui/src/lib/other-apps.ts names no app to import from');
	return kind;
}

/**
 * @typedef {object} Found what the library and the empty instance hold, found once both are up
 * @property {string} shortRecipe
 * @property {string} longRecipe
 * @property {string} shortFood
 * @property {string} longFood
 * @property {string} search a word that finds recipes
 * @property {string} oldVersion the long recipe's first Version, for a Version's own page
 * @property {string | null} importJob one import's Job, for its page, when the library holds one
 * @property {{ invite: string, kitchen: string, cookbook: string, recovery: string }} secrets each page's path
 */

/**
 * @typedef {object} Row one screen at one size in one Language
 * @property {string} screen
 * @property {string} path
 * @property {'library' | 'empty'} instance
 * @property {'everyday' | 'empty' | 'settings' | 'secret'} group
 * @property {string} size
 * @property {string} locale
 * @property {boolean} picture
 * @property {boolean} breakage the 360 px pass, which counts only what breaks
 * @property {boolean} signedOut
 * @property {boolean} touch a touch screen: every size but the MacBook, an iPad included
 * @property {'plus' | 'search'} [open] what to do once the page is open: open the + menu,
 *   or type `typed` into the search field, which no address fills
 * @property {string} [typed] the word typed into the search field
 * @property {boolean} [changesLibrary] opening it changes the library: the cooking screen starts
 *   a cooking, the share screen makes a Share Link
 * @property {string} [accent] the accent colour's token when it is not `--color-accent`
 */

/** @typedef {Omit<Row, 'size' | 'locale' | 'picture' | 'breakage' | 'touch'>} Screen */

/**
 * Every row a full run measures, in the order it measures them.
 * @param {Found} found
 * @returns {Row[]}
 */
export function coverage(found) {
	/** @type {Screen[]} */
	const screens = [
		['home', '/'],
		['recipes', '/recipes'],
		['search', '/recipes'],
		['recipe-short', `/recipes/${found.shortRecipe}`],
		['recipe-long', `/recipes/${found.longRecipe}`],
		['share', `/recipes/${found.shortRecipe}/share`],
		['plus-menu', '/recipes'],
		['cooking', `/cook/${found.shortRecipe}`],
		['shopping', '/shopping'],
		['plan', '/plan'],
		['food-short', `/foods/${found.shortFood}`],
		['food-long', `/foods/${found.longFood}`],
	].map(([screen, route]) => ({
		screen,
		path: route,
		instance: /** @type {const} */ ('library'),
		group: /** @type {const} */ ('everyday'),
		signedOut: false,
		...(screen === 'plus-menu' ? { open: /** @type {const} */ ('plus') } : {}),
		...(screen === 'search' ? { open: /** @type {const} */ ('search'), typed: found.search } : {}),
		...(screen === 'share' ? { changesLibrary: true } : {}),
		...(screen === 'cooking' ? { changesLibrary: true, accent: '--color-cook-accent' } : {}),
	}));
	const everyday = new Set(screens.map((s) => s.path));
	for (const [screen, route] of [
		['home', '/'],
		['recipes', '/recipes'],
		['shopping', '/shopping'],
		['plan', '/plan'],
	]) {
		screens.push({
			screen: `empty-${screen}`,
			path: route,
			instance: 'empty',
			group: 'empty',
			signedOut: false,
		});
	}
	for (const route of plainRoutes()) {
		if (everyday.has(route) || NOT_MEASURED.includes(route)) continue;
		screens.push({
			screen: route.slice(1).replaceAll('/', '-'),
			path: route,
			instance: 'library',
			group: 'settings',
			signedOut: false,
		});
	}
	// The pages a route parameter leads to, beyond the everyday ones.
	for (const [screen, route] of [
		['recipe-thread', `/recipes/${found.longRecipe}/thread`],
		['recipe-version', `/recipes/${found.longRecipe}/versions/${found.oldVersion}`],
		['import-from-app', `/imports/from/${firstOtherApp()}`],
		...(found.importJob ? [['import-job', `/imports/${found.importJob}`]] : []),
	]) {
		screens.push({ screen, path: route, instance: 'library', group: 'settings', signedOut: false });
	}
	for (const [screen, route] of Object.entries(found.secrets)) {
		screens.push({
			screen: `${screen}-link`,
			path: route,
			instance: 'empty',
			group: 'secret',
			signedOut: true,
		});
	}

	const all = locales();
	/** @type {Row[]} */
	const rows = [];
	for (const s of screens) {
		const wide = s.group === 'everyday' || s.group === 'empty';
		const sizes = wide ? Object.keys(EVERYDAY_SIZES) : Object.keys(AUDIT_PHONES);
		for (const locale of wide ? all : ['en']) {
			for (const size of sizes) {
				rows.push({
					...s,
					size,
					locale,
					picture: PICTURED.includes(locale),
					breakage: false,
					touch: !MOUSE_SIZES.includes(size),
				});
			}
		}
		for (const locale of PICTURED) {
			rows.push({ ...s, size: 'narrowest', locale, picture: true, breakage: true, touch: true });
		}
	}
	return rows;
}

/**
 * A consistent copy of the dev instance's data, made without disturbing it:
 * the database by SQLite's backup on a read-only connection, never a plain
 * copy of a file a running server writes, and every folder beside it but the
 * Backups. The migration snapshots are left too.
 * @param {string} from
 * @param {string} to
 */
async function copyLibrary(from, to) {
	fs.mkdirSync(to, { recursive: true });
	const source = new DatabaseSync(path.join(from, 'kamosu.db'), { readOnly: true });
	try {
		await backup(source, path.join(to, 'kamosu.db'));
	} finally {
		source.close();
	}
	for (const entry of fs.readdirSync(from, { withFileTypes: true })) {
		if (entry.isDirectory() && entry.name !== 'backups') {
			fs.cpSync(path.join(from, entry.name), path.join(to, entry.name), { recursive: true });
		}
	}
}

/** The columns a server writes on a read: when something was last used or asked. */
const STAMPS = /^last_(used|asked)_at$/;

/**
 * A fingerprint of each table in a database, read on a read-only connection:
 * every row, sorted, hashed. Two reads of an unchanged database agree. The
 * columns a running server stamps by itself when it is only asked something,
 * a Session's or a key's last use and a Runner's last poll, are left out, so
 * someone using the dev instance during a run is not taken for a change.
 * @param {string} file
 * @returns {Record<string, string>}
 */
export function tableFingerprints(file) {
	const db = new DatabaseSync(file, { readOnly: true });
	try {
		const tables = db
			.prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
			.all()
			.map((row) => String(row.name));
		/** @type {Record<string, string>} */
		const found = {};
		for (const table of tables) {
			const rows = db
				.prepare(`SELECT * FROM "${table.replaceAll('"', '""')}"`)
				.all()
				.map((row) =>
					JSON.stringify(row, (key, value) => {
						if (STAMPS.test(key)) return undefined;
						return value instanceof Uint8Array ? Buffer.from(value).toString('base64') : value;
					}),
				)
				.sort();
			found[table] = crypto.createHash('sha256').update(rows.join('\n')).digest('hex');
		}
		return found;
	} finally {
		db.close();
	}
}

// ── what the library holds ───────────────────────────────────────────────────

/**
 * The short and the long recipe, the Foods read least and most, and a word
 * that finds several recipes. Ties go to the lower id, so two runs over one
 * library choose alike.
 * @param {string} api
 * @param {string} secret
 */
async function chooseFrom(api, secret) {
	const { recipes } = await op(api, secret, 'search_recipes', {});
	/** @type {{ id: string, size: number, title: string }[]} */
	const sized = [];
	for (const { branch_id: id } of recipes) {
		const recipe = await op(api, secret, 'get_recipe', { branch_id: id });
		const head = recipe.versions.find(
			(/** @type {{ version_id: string }} */ v) => v.version_id === recipe.head_version_id,
		);
		const content = head?.content ?? {};
		/** @param {{ kind: string }[] | undefined} list @param {string} kind */
		const count = (list, kind) => (list ?? []).filter((item) => item.kind === kind).length;
		const lines = count(content.ingredients, 'ingredient');
		const steps = count(content.steps, 'step');
		if (lines > 0 && steps > 0) sized.push({ id, size: lines + steps, title: content.title ?? '' });
	}
	if (sized.length < 2)
		throw new ToolError('the dev library holds fewer than two recipes with lines and steps');
	const fewest = [...sized].sort((a, b) => a.size - b.size || a.id.localeCompare(b.id));
	const most = [...sized].sort((a, b) => b.size - a.size || a.id.localeCompare(b.id));
	const long = await op(api, secret, 'get_recipe', { branch_id: most[0].id });
	const [oldVersion] = long.versions
		.toSorted(
			(/** @type {{ sequence: number }} */ a, /** @type {{ sequence: number }} */ b) =>
				a.sequence - b.sequence,
		)
		.map((/** @type {{ version_id: string }} */ v) => v.version_id);

	const { imports } = await op(api, secret, 'list_imports', {});
	const [importJob = null] = imports
		.flatMap((/** @type {{ arrivals: { job_id: string }[] }} */ i) =>
			i.arrivals.map((a) => a.job_id),
		)
		.sort();

	const { foods } = await op(api, secret, 'list_foods', {});
	/** @type {{ reading_count: number, id: string }[]} */
	const read = foods.filter((/** @type {{ reading_count: number }} */ f) => f.reading_count > 0);
	if (read.length === 0) throw new ToolError('the dev library holds no Food any line reads as');
	const least = [...read].sort(
		(a, b) => a.reading_count - b.reading_count || a.id.localeCompare(b.id),
	);
	const oftenest = [...read].sort(
		(a, b) => b.reading_count - a.reading_count || a.id.localeCompare(b.id),
	);

	/** @type {Map<string, number>} */
	const titled = new Map();
	for (const { title } of sized) {
		for (const word of new Set(title.toLowerCase().match(/\p{L}{4,}/gu) ?? [])) {
			titled.set(word, (titled.get(word) ?? 0) + 1);
		}
	}
	const [[search]] = [...titled.entries()].sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]));
	return {
		shortRecipe: fewest[0].id,
		longRecipe: most[0].id,
		shortFood: least[0].id,
		longFood: oftenest[0].id,
		search,
		oldVersion,
		importJob,
	};
}

/**
 * The four links a stranger opens, made on the empty instance.
 * @param {string} api
 * @param {string} secret
 * @param {string} name the empty instance's first Person
 */
async function makeSecrets(api, secret, name) {
	const pathOf = (/** @type {string} */ link) => new URL(link, api).pathname;
	const invite = pathOf((await op(api, secret, 'mint_invite', {})).link);
	const kitchen = await op(api, secret, 'create_kitchen', { name: 'Home' });
	const kitchenInvite = await op(api, secret, 'invite_to_kitchen', { kitchen_id: kitchen.id });
	const cookbookInvite = await op(api, secret, 'invite_to_cookbook', {});
	const recovery = pathOf((await op(api, secret, 'mint_recovery_link', { name })).link);
	return {
		invite,
		kitchen: `/kitchen-invite/${kitchenInvite.secret}`,
		cookbook: `/cookbook-invite/${cookbookInvite.secret}`,
		recovery,
	};
}

// ── measuring ────────────────────────────────────────────────────────────────

/** How long a screen must be still before it is counted: the research note's two seconds. */
const STILL_MS = 2000;

/**
 * @typedef {object} Place where one instance's rows are measured
 * @property {string} origin the instance as the browser opens it
 * @property {string} secret its Session's Secret
 * @property {string} out the folder written into
 * @property {string[]} faults what went wrong, for the end of the run
 */

/**
 * Count one row's screen in a page already open on its context. `answered`
 * is filled with what the instance answers from the moment the page is
 * opened: the research note's "each JSON answer the page receives while it
 * loads".
 * @param {import('playwright').Page} page
 * @param {Row} row
 * @param {Place & { answered: Set<string>, pending: Set<Promise<void>> }} at
 */
async function measureRow(page, row, at) {
	const where = `${row.screen} at ${row.size} in ${row.locale}`;
	const unsettled = await openSettled(page, at.origin + row.path, { quiet: STILL_MS });
	if (row.open === 'plus') {
		await page
			.getByRole('button', { name: phrases(row.locale).plus_label })
			.first()
			.click();
	}
	if (row.open === 'search') {
		await page
			.locator('input[type=search]')
			.first()
			.fill(row.typed ?? '');
	}
	if (row.open) {
		await page.waitForLoadState('networkidle');
		await page.waitForTimeout(STILL_MS);
	}
	await hideBrowserCards(page, row.locale);
	await Promise.all(at.pending);

	let picture = '';
	if (row.picture) {
		const folder = path.join(at.out, 'pictures', row.locale);
		fs.mkdirSync(folder, { recursive: true });
		try {
			const { files } = await photograph(page, folder, row.screen, row.size);
			picture = files.map((file) => path.relative(at.out, file)).join(' ');
		} catch (error) {
			at.faults.push(`${where}: not photographed: ${String(error).split('\n')[0]}`);
		}
	}
	// The cooking screen's own accent stands in for the accent there, and
	// nowhere else.
	const accents = await page.evaluate(
		(name) =>
			[getComputedStyle(document.documentElement).getPropertyValue(name).trim()].filter(Boolean),
		row.accent ?? '--color-accent',
	);
	const counted = await page.evaluate(measureInPage, {
		values: [...at.answered],
		accents,
		probe: !row.breakage,
		breakage: row.breakage,
	});
	return { unsettled, picture, ...(row.breakage ? { broken: counted } : { measures: counted }) };
}

/** The table's columns, each read off a row's result. */
const COLUMNS = /** @type {[string, (r: any) => unknown][]} */ ([
	['screen', (r) => r.screen],
	['path', (r) => r.path],
	['instance', (r) => r.instance],
	['size', (r) => r.size],
	['language', (r) => r.locale],
	['interface_words_first', (r) => r.measures?.words.interfaceFirst],
	['interface_words_page', (r) => r.measures?.words.interfacePage],
	['content_words_first', (r) => r.measures?.words.contentFirst],
	['content_words_page', (r) => r.measures?.words.contentPage],
	['explanations_first', (r) => r.measures?.words.explanationsFirst.length],
	['explanations_page', (r) => r.measures?.words.explanationsPage.length],
	['sentences_over_25_words', (r) => r.measures?.words.long.length],
	['room_percent', (r) => r.measures && Math.round(r.measures.room * 1000) / 10],
	['controls_first', (r) => r.measures?.controls.first],
	['main_buttons_first', (r) => r.measures?.controls.mainButtons.length],
	['unmarked_actions', (r) => r.measures?.unmarked.length],
	['taps_under_48', (r) => r.measures?.tap.small.length],
	['taps_cut_short', (r) => r.measures?.tap.cutShort.length],
	['taps_covered', (r) => r.measures?.tap.covered.length],
	['tapped_areas_overlapping', (r) => r.measures?.gaps.overlapping.length],
	['smallest_gap_px', (r) => r.measures?.gaps.smallest?.px],
	['font_sizes', (r) => r.measures?.type.sizes.length],
	['text_colours', (r) => r.measures?.type.colours.length],
	['screenfuls', (r) => r.measures?.length.screenfuls],
	['main_action_at', (r) => r.measures?.length.mainActionAt],
	['notices', (r) => r.measures?.notices],
	['text_cut_off', (r) => r.broken?.cutOff.length],
	['boxes_overlapping', (r) => r.broken?.overlaps.length],
	['sideways_px', (r) => r.broken?.sideways],
	['unsettled', (r) => r.unsettled ?? ''],
	['picture', (r) => r.picture],
]);

/**
 * One table, as CSV: a header, then a line per row.
 * @param {object[]} results
 */
export function tableOf(results) {
	const cell = (/** @type {unknown} */ value) => {
		const text = value === undefined || value === null ? '' : String(value);
		return /[",\n]/.test(text) ? `"${text.replaceAll('"', '""')}"` : text;
	};
	return [
		COLUMNS.map(([name]) => name).join(','),
		...results.map((result) => COLUMNS.map(([, read]) => cell(read(result))).join(',')),
	]
		.join('\n')
		.concat('\n');
}

/**
 * Open the screens that change the library once before anything is counted:
 * the cooking screen starts a cooking and the share screen makes a Share
 * Link. Every row then sees the library as it stands after both, rather than
 * the first size and Language seeing it before and the rest after.
 * @param {import('playwright').Browser} browser
 * @param {Row[]} rows
 * @param {Place} at
 */
async function settleWhatRowsChange(browser, rows, at) {
	const changing = [...new Set(rows.filter((r) => r.changesLibrary).map((r) => r.path))];
	if (changing.length === 0) return;
	const context = await signedInContext(
		browser,
		at.origin,
		at.secret,
		AUDIT_PHONES['phone-402'],
		'en',
		{
			touch: true,
		},
	);
	try {
		const page = await context.newPage();
		for (const route of changing) await openSettled(page, at.origin + route, { quiet: STILL_MS });
	} finally {
		await context.close();
	}
}

/**
 * Measure rows on one instance, one browser context per size and Language.
 * Results are pushed onto `results` as they come, so a run that fails late
 * still writes what it measured.
 * @param {import('playwright').Browser} browser
 * @param {Row[]} rows
 * @param {Place} at
 * @param {object[]} results
 */
async function measureOn(browser, rows, at, results) {
	/** @type {Map<string, Row[]>} */
	const byContext = new Map();
	for (const row of rows) {
		const key = `${row.size} ${row.locale} ${row.signedOut}`;
		byContext.set(key, [...(byContext.get(key) ?? []), row]);
	}
	for (const group of byContext.values()) {
		const { size, locale, signedOut, touch } = group[0];
		const dimensions = ALL_SIZES[/** @type {keyof typeof ALL_SIZES} */ (size)];
		const context = signedOut
			? await toolContext(browser, dimensions, locale, { touch })
			: await signedInContext(browser, at.origin, at.secret, dimensions, locale, { touch });
		const state = { answered: new Set(), pending: new Set() };
		context.on('response', (response) => {
			if (!new URL(response.url()).pathname.startsWith('/api/op/')) return;
			const into = state.answered;
			/** @type {Promise<void>} */
			const read = response
				.json()
				.then((answer) => void valuesOf(answer, into))
				.catch(() => {})
				.finally(() => state.pending.delete(read));
			state.pending.add(read);
		});
		try {
			const page = await context.newPage();
			if (!signedOut) {
				// A browser that did not send the Session would count the
				// sign-in page in place of every screen, and say nothing.
				const answer = await (
					await page.request.post(`${at.origin}/api/op/get_person`, { data: {} })
				).json();
				if (!answer?.ok)
					throw new ToolError(
						`the browser at ${size} in ${locale} is not signed in to ${at.origin}`,
					);
			}
			page.on('pageerror', (error) =>
				at.faults.push(`${new URL(page.url()).pathname} at ${size} in ${locale}: ${error}`),
			);
			for (const row of group) {
				const started = Date.now();
				state.answered = new Set();
				state.pending = new Set();
				try {
					const result = await measureRow(page, row, { ...at, ...state });
					results.push({ ...row, ...result, seconds: (Date.now() - started) / 1000 });
				} catch (error) {
					const why = `not measured: ${String(error).split('\n')[0]}`;
					at.faults.push(`${row.screen} at ${size} in ${locale}: ${why}`);
					results.push({ ...row, unsettled: why });
				}
				console.log(`${row.screen} ${size} ${locale}${row.breakage ? ' (breakage)' : ''}`);
			}
		} finally {
			await context.close();
		}
	}
}

/**
 * A length of time, given in milliseconds, as minutes and seconds.
 * @param {number} ms
 */
const took = (ms) => `${Math.floor(ms / 60000)}m ${Math.round((ms % 60000) / 1000)}s`;

async function main() {
	const began = Date.now();
	const args = process.argv.slice(2);
	const everydayOnly = args.includes('--everyday');
	const binaryAt = args.indexOf('--binary');
	const binary = binaryAt >= 0 ? args.splice(binaryAt, 2)[1] : undefined;
	const [folder] = args.filter((arg) => !arg.startsWith('--'));
	if (!folder || !binary) throw new ToolError('usage: just ux-measure <folder> [--everyday]');
	const out = fromRoot(folder);
	fs.mkdirSync(out, { recursive: true });

	const env = devEnv();
	const account = {
		name: need(env, 'KAMOSU_DEV_ADMIN_NAME'),
		password: need(env, 'KAMOSU_DEV_ADMIN_PASSWORD'),
	};
	const devData = path.join(ROOT, '.dev', 'data');
	const devDb = path.join(devData, 'kamosu.db');
	if (!fs.existsSync(devDb))
		throw new ToolError(`there is no dev library at ${path.relative(ROOT, devDb)} to copy`);
	const before = tableFingerprints(devDb);

	const scratch = throwawayFolder('ux-measure');
	/** @type {(() => Promise<void>)[]} */
	const stops = [];
	/** @type {string[]} */
	const faults = [];
	/** @type {object[]} */
	const results = [];
	try {
		await copyLibrary(devData, path.join(scratch, 'library'));
		const library = await startInstance(
			binary,
			path.join(scratch, 'library'),
			path.join(out, 'library.log'),
		);
		stops.push(library.stop);
		fs.mkdirSync(path.join(scratch, 'empty'));
		const empty = await startInstance(
			binary,
			path.join(scratch, 'empty'),
			path.join(out, 'empty.log'),
		);
		stops.push(empty.stop);

		const librarySecret = await signInAt(`${library.api}/auth/login`, {
			...account,
			session_name: 'ux-measure',
		});
		const firstPerson = {
			name: 'Stéphane',
			password: crypto.randomBytes(18).toString('base64url'),
		};
		const emptySecret = await signInAt(`${empty.api}/auth/first-person`, {
			...firstPerson,
			session_name: 'ux-measure',
		});
		const found = {
			...(await chooseFrom(library.api, librarySecret)),
			secrets: await makeSecrets(empty.api, emptySecret, firstPerson.name),
		};
		fs.writeFileSync(path.join(out, 'chosen.json'), `${JSON.stringify(found, null, '\t')}\n`);

		const rows = coverage(found).filter(
			(row) => !everydayOnly || ['everyday', 'empty'].includes(row.group),
		);
		console.log(`measuring ${rows.length} rows on two throwaway instances`);
		const browser = await openBrowser();
		try {
			/** @type {[Place, Row[]][]} */
			const places = [
				[
					{ origin: library.origin, secret: librarySecret, out, faults },
					rows.filter((r) => r.instance === 'library'),
				],
				[
					{ origin: empty.origin, secret: emptySecret, out, faults },
					rows.filter((r) => r.instance === 'empty'),
				],
			];
			for (const [place, theirs] of places) {
				await settleWhatRowsChange(browser, theirs, place);
				await measureOn(browser, theirs, place, results);
			}
		} finally {
			await browser.close();
		}
	} finally {
		await Promise.all(stops.map((stop) => stop().catch(() => {})));
		leaveNothingNow();
		fs.writeFileSync(path.join(out, 'details.json'), `${JSON.stringify(results, null, '\t')}\n`);
		fs.writeFileSync(path.join(out, 'measures.csv'), tableOf(results));
		console.log(
			`wrote ${path.relative(ROOT, path.join(out, 'measures.csv'))}: ${results.length} rows`,
		);
		const after = tableFingerprints(devDb);
		const changed = Object.keys({ ...before, ...after }).filter(
			(table) => before[table] !== after[table],
		);
		if (changed.length) {
			faults.push(
				`the dev database changed during the run, in ${changed.join(', ')}: this tool writes nothing there, so something else did`,
			);
		} else {
			console.log('the dev database is unchanged');
		}
		console.log(`run time: ${took(Date.now() - began)}`);
	}
	if (faults.length) throw new ToolError(`${faults.length} fault(s):\n${faults.join('\n')}`);
}

if (process.argv[1] === import.meta.filename) await run(main);
