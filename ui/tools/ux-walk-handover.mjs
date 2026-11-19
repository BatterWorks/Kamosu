/**
 * The hand-over command of `just ux-walk` (#639), copied into an evaluator's
 * empty folder as `walk.mjs` beside `walk-pack.json`, which holds each task's
 * goal and pictures.
 *
 * It hands an evaluator a task's pictures one at a time. Each picture is
 * written out only when asked for, and the next is given only once the last
 * has a guess: what the evaluator would tap, why, and anything unclear. Every
 * guess is kept, in order, in `guesses.jsonl`, which the audit reads back.
 * The pictures not yet handed over stay inside the pack, where no listing
 * of the folder shows them. The evaluator is a model told not to look ahead,
 * so this keeps the order honest rather than locked.
 *
 * It runs on Node alone, with no Kamosu and no repo around it, and names
 * nothing the pictures do not show: no file of the repo is read and no word
 * of Kamosu's own is written here.
 */

import fs from 'node:fs';
import path from 'node:path';

const HERE = process.cwd();
const PACK = path.join(HERE, 'walk-pack.json');
const GUESSES = path.join(HERE, 'guesses.jsonl');
const SHOWN = path.join(HERE, 'pictures');

/** Something the evaluator can put right. Printed without a stack. */
class Refused extends Error {}

/**
 * @typedef {object} PackedTask a task as the pack holds it
 * @property {string} id
 * @property {string} goal
 * @property {string[]} pictures each picture's PNG, in base64, in the order it is handed over
 */

/** @returns {PackedTask[]} */
function tasks() {
	if (!fs.existsSync(PACK)) throw new Refused(`there is no walk-pack.json in ${HERE}`);
	return JSON.parse(fs.readFileSync(PACK, 'utf8')).tasks;
}

/** @param {string} id */
function task(id) {
	const found = tasks().find((t) => t.id === id);
	if (!found) {
		throw new Refused(
			`there is no task "${id}". The tasks are: ${tasks()
				.map((t) => t.id)
				.join(', ')}`,
		);
	}
	return found;
}

/**
 * @typedef {object} Guess
 * @property {string} task
 * @property {number} picture counted from 1
 * @property {string} tap
 * @property {string} why
 * @property {string} unclear
 * @property {string} at
 */

/** @returns {Guess[]} */
function guesses() {
	if (!fs.existsSync(GUESSES)) return [];
	return fs
		.readFileSync(GUESSES, 'utf8')
		.split('\n')
		.filter((line) => line.trim())
		.map((line) => JSON.parse(line));
}

/** @param {string} id @param {number} picture */
const shownFile = (id, picture) => path.join(SHOWN, `${id}-${picture}.png`);

/** How many of a task's pictures have been handed over. */
function shownCount(/** @type {string} */ id) {
	let count = 0;
	while (fs.existsSync(shownFile(id, count + 1))) count++;
	return count;
}

const guessedCount = (/** @type {string} */ id) => guesses().filter((g) => g.task === id).length;

function list() {
	console.log('The tasks, each with its goal:\n');
	for (const t of tasks()) {
		const shown = shownCount(t.id);
		const state = shown === 0 ? 'not started' : `${shown} picture(s) seen`;
		console.log(`  ${t.id}: ${t.goal} (${state})`);
	}
	console.log(`
Start a task with: node walk.mjs next <task>
Each picture is what a phone shows at that moment. Say what you would tap or
type next to reach the goal, why, and anything unclear, before asking for the
next picture:
  node walk.mjs guess <task> --tap "..." --why "..." [--unclear "..."]
When a picture shows the goal reached, say so in --tap.`);
}

/** @param {string} id */
function next(id) {
	const t = task(id);
	const shown = shownCount(id);
	if (shown > guessedCount(id)) {
		throw new Refused(
			`picture ${shown} of ${id} has no guess yet: write it first with node walk.mjs guess ${id} --tap "..." --why "..."`,
		);
	}
	if (shown === t.pictures.length) {
		console.log(`No more pictures for ${id}: its walk is over.`);
		return;
	}
	const file = shownFile(id, shown + 1);
	fs.mkdirSync(SHOWN, { recursive: true });
	fs.writeFileSync(file, Buffer.from(t.pictures[shown], 'base64'));
	console.log(`Task: ${id}
Goal: ${t.goal}
Picture ${shown + 1}: ${path.relative(HERE, file)}

Look at it, then say what you would tap or type next, why, and anything unclear:
  node walk.mjs guess ${id} --tap "..." --why "..." [--unclear "..."]
If this picture shows the goal reached, say so in --tap.`);
}

/**
 * @param {string} id
 * @param {string[]} rest
 */
function guess(id, rest) {
	task(id);
	/** @type {Record<string, string>} */
	const said = {};
	for (let i = 0; i < rest.length; i += 2) {
		const flag = rest[i];
		if (!['--tap', '--why', '--unclear'].includes(flag) || rest[i + 1] === undefined) {
			throw new Refused(`a guess takes --tap "...", --why "..." and maybe --unclear "..."`);
		}
		said[flag.slice(2)] = rest[i + 1];
	}
	if (!said.tap?.trim() || !said.why?.trim()) {
		throw new Refused('a guess says what you would tap (--tap) and why (--why)');
	}
	const shown = shownCount(id);
	const guessed = guessedCount(id);
	if (shown === 0)
		throw new Refused(`no picture of ${id} has been handed over: node walk.mjs next ${id}`);
	if (guessed === shown) {
		throw new Refused(`picture ${shown} of ${id} already has a guess: node walk.mjs next ${id}`);
	}
	/** @type {Guess} */
	const kept = {
		task: id,
		picture: shown,
		tap: said.tap,
		why: said.why,
		unclear: said.unclear ?? '',
		at: new Date().toISOString(),
	};
	fs.appendFileSync(GUESSES, `${JSON.stringify(kept)}\n`);
	console.log(`Kept your guess for picture ${shown} of ${id}. Next: node walk.mjs next ${id}`);
}

try {
	const [verb, id, ...rest] = process.argv.slice(2);
	if (!verb) list();
	else if (verb === 'next' && id) next(id);
	else if (verb === 'guess' && id) guess(id, rest);
	else
		throw new Refused('usage: node walk.mjs [next <task> | guess <task> --tap "..." --why "..."]');
} catch (error) {
	if (!(error instanceof Refused)) throw error;
	console.error(`refused: ${error.message}`);
	process.exitCode = 1;
}
